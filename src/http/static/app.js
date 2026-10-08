let offset = 0;
const LIMIT = 20;
const proj = () => document.getElementById('project').value.trim() || 'default';

function tab(name, btn) {
    document.querySelectorAll('.tabs button').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    ['browse', 'search', 'add', 'snaps'].forEach(t => document.getElementById('pane-' + t).classList.toggle('hidden', t !== name));
    if (name === 'snaps') loadSnaps();
}
async function api(path, opts) {
    const r = await fetch(path, opts);
    const j = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(j.error || r.statusText);
    return j;
}
async function refreshAll() {
    offset = 0;
    await Promise.all([loadHealth(), loadStats(), loadPoints()]);
}
async function loadHealth() {
    try {
        const h = await api('/healthz');
        document.getElementById('health').textContent = h.status || 'ok';
        const v = await api('/api/version');
        document.getElementById('ver').textContent = 'v' + v.version + ' | ' + v.model;
    } catch (e) {
        document.getElementById('health').textContent = 'offline';
    }
}
async function loadStats() {
    try {
        const s = await api('/api/collections/stats?project=' + encodeURIComponent(proj()));
        document.getElementById('collStats').innerHTML = '<div class="stat"><span>points</span><b>' + s.points_count + '</b></div>';
        const pr = s.per_project && Object.keys(s.per_project).length ? s.per_project : {
            [s.project]: s.points_count
        };
        document.getElementById('projects').innerHTML = Object.entries(pr).map(([k, v]) => '<div class="stat"><span>' + k + '</span><b>' + v + '</b></div>').join('');
        document.getElementById('cats').innerHTML = Object.entries(s.per_category || {}).map(([k, v]) => '<div class="stat"><span>' + k + '</span><b>' + v + '</b></div>').join('');
    } catch (e) {}
}
async function loadPoints() {
    try {
        const q = new URLSearchParams({
            project: proj(),
            limit: LIMIT,
            offset
        });
        if (document.getElementById('fq').value) q.set('query', document.getElementById('fq').value);
        if (document.getElementById('fcat').value) q.set('category', document.getElementById('fcat').value);
        const d = await api('/api/points?' + q);
        document.getElementById('pageInfo').textContent = offset + ' of ' + d.total;
        let html = '<table><tr><th>id</th><th>category</th><th>content</th><th></th></tr>';
        d.points.forEach(p => {
            html += '<tr><td>' + esc(p.id) + '</td><td>' + esc(p.category) + '</td><td class="content">' + esc(p.content) + '</td><td><button class="danger" data-id="' + esc(p.id) + '">x</button></td></tr>';
        });
        document.getElementById('points').innerHTML = html + '</table>';
        document.querySelectorAll('#points button.danger').forEach(b => b.onclick = () => delPoint(b.dataset.id));
    } catch (e) {
        document.getElementById('points').textContent = 'error: ' + e.message;
    }
}

function page(d) {
    offset = Math.max(0, offset + d * LIMIT);
    loadPoints();
}
async function delPoint(id) {
    if (!confirm('Delete ' + id + '?')) return;
    await api('/api/points/' + encodeURIComponent(id) + '?project=' + encodeURIComponent(proj()), {
        method: 'DELETE'
    });
    loadPoints();
    loadStats();
}
async function doSearch() {
    const box = document.getElementById('hits');
    box.innerHTML = 'searching...';
    try {
        const h = await api('/api/points/search', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                query: document.getElementById('sq').value,
                project: proj(),
                limit: 10
            })
        });
        box.innerHTML = h.map((m, i) => '<div class="card"><b>' + (i + 1) + '. ' + esc(m.id) + '</b><br/>' + esc(m.content) + '</div>').join('') || 'no matches';
    } catch (e) {
        box.textContent = 'error: ' + e.message;
    }
}
async function doUpsert() {
    const out = document.getElementById('upsertOut');
    out.classList.remove('hidden');
    out.textContent = 'upserting...';
    try {
        const r = await api('/api/points/upsert', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                id: document.getElementById('uid').value,
                category: document.getElementById('ucat').value,
                content: document.getElementById('ucontent').value,
                project: proj()
            })
        });
        out.textContent = JSON.stringify(r, null, 2);
        loadPoints();
        loadStats();
    } catch (e) {
        out.textContent = 'error: ' + e.message;
    }
}
async function optimize() {
    await api('/api/optimize', {
        method: 'POST'
    });
    alert('optimized');
}
async function loadSnaps() {
    const box = document.getElementById('snaps');
    try {
        const s = await api('/api/snapshots');
        let html = '<table><tr><th>name</th><th>size</th><th>created</th><th></th></tr>';
        s.forEach(x => {
            html += '<tr><td>' + x.name + '</td><td>' + (x.size_bytes / 1024).toFixed(1) + ' KB</td><td>' + x.created + '</td><td><button class="ghost" data-re="' + x.name + '">restore</button> <button class="danger" data-rm="' + x.name + '">x</button></td></tr>';
        });
        box.innerHTML = html + '</table>';
        box.querySelectorAll('[data-re]').forEach(b => b.onclick = () => reSnap(b.dataset.re));
        box.querySelectorAll('[data-rm]').forEach(b => b.onclick = () => rmSnap(b.dataset.rm));
    } catch (e) {
        box.textContent = 'error: ' + e.message;
    }
}
async function mkSnap() {
    await api('/api/snapshots', {
        method: 'POST'
    });
    loadSnaps();
}
async function rmSnap(n) {
    if (!confirm('Delete ' + n + '?')) return;
    await api('/api/snapshots/' + n, {
        method: 'DELETE'
    });
    loadSnaps();
}
async function reSnap(n) {
    if (!confirm('Restore ' + n + '? DB will be replaced.')) return;
    await api('/api/snapshots/' + n + '/restore', {
        method: 'POST'
    });
    refreshAll();
}

function esc(s) {
    return String(s || '').replace(/[&<>"]/g, c => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;'
    } [c]));
}
refreshAll();