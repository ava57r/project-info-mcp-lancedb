mod helpers;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_stream::StreamExt;

use arrow_array::RecordBatch;
use arrow_array::cast::AsArray;
use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::message::{Context, Message};
use lancedb::index::{Index, scalar::FtsIndexBuilder};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, table::Table};
use mcp_sdk_rs::error::{Error as McpError, ErrorCode};
use mcp_sdk_rs::server::{Server, ServerHandler};
use mcp_sdk_rs::transport::{Transport, stdio::StdioTransport};
use mcp_sdk_rs::types::{
    ClientCapabilities, Implementation, ListToolsResult, MessageContent, ServerCapabilities, Tool,
    ToolResult, ToolSchema,
};
use reqwest::Client;

// Константы окружения
const OPENVINO_EMBED_URL: &str = "http://localhost:8001/v1/embeddings";
const MODEL_NAME: &str = "qwen";
const DB_DIR: &str = "./.opencode_memory/kameo_db";

// --- Сообщения для Актора (Соответствуют подзадачам MCP) ---

#[derive(Serialize, Deserialize)]
pub struct UpsertMessage {
    pub id: String,
    pub content: String,
    pub category: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchMessage {
    pub query: String,
    pub category: Option<String>,
    pub limit: usize,
}

pub struct OptimizeMessage;

// --- Определение Актора Памяти ---

struct MemoryActor {
    table: Table,
    http_client: Client,
}

impl Actor for MemoryActor {
    type Args = MemoryActor;

    type Error = anyhow::Error;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(args)
    }
}

impl Message<UpsertMessage> for MemoryActor {
    type Reply = Result<String, String>;

    async fn handle(
        &mut self,
        msg: UpsertMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // 1. Вычисляем SHA-256 хеш нового контента
        let mut hasher = Sha256::new();
        hasher.update(msg.content.as_bytes());
        let current_hash = hex::encode(hasher.finalize());

        // 2. Ищем существующую запись по `id` на уровне SQL LanceDB
        // Используем точечный фильтр `only_if`, чтобы не читать всю базу в память
        let predicate = format!("id = '{}'", msg.id);
        if let Ok(mut stream) = self
            .table
            .query()
            .only_if(&predicate)
            .limit(1)
            .execute()
            .await
        {
            // Читаем батчи из потока Arrow данных
            // В новых версиях lancedb это асинхронный поток (Stream), перебираем его
            use tokio_stream::StreamExt; // Убедитесь, что этот трейт доступен, или используйте обычный .next()

            if let Some(Ok(batch)) = stream.next().await
                && batch.num_rows() > 0
            {
                // Находим индекс колонки "file_hash" в схеме
                if let Ok(hash_col_idx) = batch.schema().index_of("file_hash") {
                    // Безопасно приводим абстрактный массив к строковому типу Arrow (StringArray)
                    let hash_array = batch.column(hash_col_idx).as_string::<i32>();

                    // Извлекаем значение первой строки (индекс 0)
                    let old_hash = hash_array.value(0);

                    // Главная проверка: если хеши совпадают, завершаем подзадачу!
                    if old_hash == current_hash {
                        return Ok(format!(
                            "ℹ️ [Kameo] Данные для '{}' не изменились (хеш совпадает). Инференс OpenVINO GPU пропущен.",
                            msg.id
                        ));
                    }
                }

                // Если хеш изменился, удаляем старую запись, чтобы избежать дубликатов при поиске
                let _ = self.table.delete(&predicate).await;
            }
        }

        // 3. Если записи нет или контент изменился -> делаем инференс эмбеддинга через OpenVINO (Intel GPU)
        let vector = get_embedding_from_ovms(&self.http_client, &msg.content).await?;

        // 4. Собираем новый бинарный RecordBatch с помощью нашей функции
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let record_batch = helpers::build_arrow_record(
            &msg.id,
            &msg.content,
            &msg.category,
            &current_hash,
            timestamp,
            vector,
        )?;

        // 5. Записываем обновленные данные в LanceDB
        self.table
            .add(record_batch)
            .execute()
            .await
            .map_err(|e| format!("Ошибка записи в LanceDB: {}", e))?;

        Ok(format!(
            "✅ [Kameo] Данные '{}' успешно обновлены в памяти проекта.",
            msg.id
        ))
    }
}

// --- Реализация обработки подзадачи: Гибридный поиск ---
impl Message<SearchMessage> for MemoryActor {
    type Reply = Result<String, String>;

    async fn handle(
        &mut self,
        msg: SearchMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let query_vector = get_embedding_from_ovms(&self.http_client, &msg.query).await?;

        // Безопасное обновление полнотекстового индекса внутри актора
        let _ = self
            .table
            .create_index(&["content"], Index::FTS(FtsIndexBuilder::default()))
            .execute()
            .await;

        let mut query_builder = self
            .table
            .query()
            .nearest_to(query_vector)
            .map_err(|e| e.to_string())?
            .limit(msg.limit);
        if let Some(cat) = msg.category {
            query_builder = query_builder.only_if(format!("category = '{}'", cat));
        }

        let mut stream = query_builder
            .execute()
            .await
            .map_err(|e| format!("Ошибка гибридного поиска: {}", e))?;
        let mut num_batches = 0usize;
        while let Some(batch) = stream.next().await {
            batch.map_err(|e| e.to_string())?;
            num_batches += 1;
        }

        // Формирование ответа для ИИ-агента
        Ok(format!(
            "🔍 Подзадача Kameo выполнена. Найдено фрагментов Arrow: {}",
            num_batches
        ))
    }
}

// --- Вспомогательные функции инференса и Arrow сборки ---

async fn get_embedding_from_ovms(client: &Client, text: &str) -> Result<Vec<f32>, String> {
    #[derive(Serialize)]
    struct Req<'a> {
        input: &'a str,
        model: &'a str,
    }
    #[derive(Deserialize)]
    struct Data {
        embedding: Vec<f32>,
    }
    #[derive(Deserialize)]
    struct Res {
        data: Vec<Data>,
    }

    let response = client
        .post(OPENVINO_EMBED_URL)
        .json(&Req {
            input: text,
            model: MODEL_NAME,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let parsed: Res = response.json().await.map_err(|e| e.to_string())?;
    parsed
        .data
        .first()
        .map(|d| d.embedding.clone())
        .ok_or_else(|| "Пустой вектор".to_string())
}

// --- MCP-обработчик: инструмент upsert_project_info поверх актора Kameo ---

/// Реализация [`ServerHandler`]: отдаёт список инструментов и вызывает актора.
struct MemoryToolHandler {
    actor: ActorRef<MemoryActor>,
}

#[async_trait]
impl ServerHandler for MemoryToolHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, McpError> {
        Ok(ServerCapabilities {
            tools: Some(serde_json::json!({})),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        match method {
            "tools/list" => {
                let tool = Tool {
                    name: "upsert_project_info".to_string(),
                    description: "Сохраняет подзадачу или контекст через актора Kameo".to_string(),
                    input_schema: Some(ToolSchema {
                        properties: Some(serde_json::json!({
                            "info_id": {
                                "type": "string",
                                "description": "Идентификатор записи"
                            },
                            "content": {
                                "type": "string",
                                "description": "Текст подзадачи или контекста"
                            },
                            "category": {
                                "type": "string",
                                "description": "Категория записи"
                            },
                        })),
                        required: Some(vec![
                            "info_id".to_string(),
                            "content".to_string(),
                            "category".to_string(),
                        ]),
                    }),
                    annotations: None,
                };
                let result = ListToolsResult {
                    tools: vec![tool],
                    next_cursor: None,
                };
                Ok(serde_json::to_value(result)?)
            }
            "tools/call" => {
                let params = params.unwrap_or_else(|| serde_json::json!({}));
                let tool_name = params
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if tool_name != "upsert_project_info" {
                    return Err(McpError::protocol(
                        ErrorCode::InvalidParams,
                        format!("unknown tool: {tool_name}"),
                    ));
                }
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                let msg = UpsertMessage {
                    id: required_arg(&args, "info_id")?,
                    content: required_arg(&args, "content")?,
                    category: required_arg(&args, "category")?,
                };
                // Асинхронный вызов актора методом `.ask()`
                let text = match self.actor.ask(msg).await {
                    Ok(text) => text,
                    Err(_) => "❌ Сбой подзадачи в акторе Kameo".to_string(),
                };
                let result = ToolResult {
                    content: vec![MessageContent::Text { text }],
                    structured_content: None,
                };
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(McpError::protocol(
                ErrorCode::MethodNotFound,
                format!("method not found: {method}"),
            )),
        }
    }
}

/// Извлекает обязательный строковый аргумент инструмента.
fn required_arg(args: &serde_json::Value, key: &str) -> Result<String, McpError> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            McpError::protocol(ErrorCode::InvalidParams, format!("missing argument: {key}"))
        })
}

// --- Точка входа в приложение и биндинг к MCP ---

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация LanceDB соединения
    let db_conn = connect(DB_DIR).execute().await?;

    // Схема таблицы (полная схема под данные памяти проекта)
    let table = match db_conn.open_table("project_memory").execute().await {
        Ok(t) => t,
        Err(_) => {
            db_conn
                .create_table(
                    "project_memory",
                    RecordBatch::new_empty(helpers::table_schema()),
                )
                .execute()
                .await?
        }
    };

    // Запускаем Актора Памяти через систему Kameo
    let memory_actor_ref = MemoryActor::spawn(MemoryActor {
        table,
        http_client: Client::new(),
    });

    // Инициализация MCP сервера (stdio-транспорт поверх каналов tokio)
    let (read_tx, read_rx) = tokio::sync::mpsc::channel::<String>(64);
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<String>(64);

    // stdin -> канал чтения транспорта
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if read_tx.send(line).await.is_err() {
                break;
            }
        }
    });

    // канал записи транспорта -> stdout
    tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        let mut stdout = tokio::io::stdout();
        while let Some(line) = write_rx.recv().await {
            let failed = stdout.write_all(line.as_bytes()).await.is_err()
                || stdout.write_all(b"\n").await.is_err()
                || stdout.flush().await.is_err();
            if failed {
                break;
            }
        }
    });

    let transport = Arc::new(StdioTransport::new(read_rx, write_tx)) as Arc<dyn Transport>;
    let handler = Arc::new(MemoryToolHandler {
        actor: memory_actor_ref,
    });
    let server = Server::new(transport, handler);
    server.start().await?;
    Ok(())
}
