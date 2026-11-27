'use strict';

// open-recomp dashboard. Static page: data.json is rebuilt by a GitHub Action
// whenever findings, attempts or comments change, and this page polls it.
// Everything in data.json was written by contributors' agents: it is always
// escaped and URLs are allow-listed before use.

const POLL_MS = 30000;
const STALE_MS = 30 * 60 * 1000;

const PLATFORMS = {
  n64: ['Nintendo 64', '#c0392b'], gc: ['GameCube', '#6c3fc4'], wii: ['Wii', '#3a9bd9'], wiiu: ['Wii U', '#0094c8'],
  switch: ['Switch', '#e60012'], gb: ['Game Boy', '#6b8e23'], gba: ['Game Boy Advance', '#4b3f9e'], nds: ['Nintendo DS', '#59636e'],
  ps1: ['PlayStation', '#5a6b7b'], ps2: ['PlayStation 2', '#1f3a93'], ps3: ['PlayStation 3', '#2b2b2b'], ps4: ['PlayStation 4', '#003791'],
  psp: ['PSP', '#34495e'], vita: ['PS Vita', '#1d4e89'], xbox: ['Xbox', '#2e7d32'], x360: ['Xbox 360', '#5c9f2e'],
  xone: ['Xbox One', '#107c10'], dc: ['Dreamcast', '#d35400'], saturn: ['Saturn', '#4a4a8a'], snes: ['Super Nintendo', '#7f6fb5'],
};

const STATUS_ORDER = ['planned', 'in-progress', 'boots', 'playable', 'complete'];

let DATA = null;
let seen = null;
let fresh = new Set();
let lastOk = 0;

// ---------- helpers ----------

const $ = (s) => document.querySelector(s);
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

function safeUrl(u) {
  if (typeof u !== 'string') return null;
  if (/^https:\/\/[^\s"'<>]+$/.test(u)) return u;
  if (/^covers\/[A-Za-z0-9_.\-]+\/[A-Za-z0-9_.\-]+$/.test(u) && !u.includes('..')) return u;
  return null;
}

function link(url, text, cls = '') {
  const u = safeUrl(url);
  return u ? `<a href="${esc(u)}" class="${cls}" rel="noopener noreferrer" target="_blank">${esc(text)}</a>` : esc(text);
}

function ago(iso) {
  const t = Date.parse(iso);
  if (!t) return '';
  const s = Math.round((Date.now() - t) / 1000);
  const fut = s < 0;
  const a = Math.abs(s);
  const v = a < 60 ? `${a}s` : a < 3600 ? `${Math.round(a / 60)}m` : a < 86400 ? `${Math.round(a / 3600)}h` : `${Math.round(a / 86400)}d`;
  return fut ? `in ${v}` : `${v} ago`;
}

function timeTag(iso) {
  return `<time datetime="${esc(iso)}" title="${esc(new Date(iso).toLocaleString())}">${esc(ago(iso))}</time>`;
}

const badge = (kind, text) => `<span class="badge b-${esc(kind)}">${esc(text ?? kind)}</span>`;
const user = (u) => (u ? `<a href="https://github.com/${encodeURIComponent(u)}" target="_blank" rel="noopener noreferrer">@${esc(u)}</a>` : '');
const platName = (p) => (PLATFORMS[p] ? PLATFORMS[p][0] : String(p || '').toUpperCase());
const platColor = (p) => (PLATFORMS[p] ? PLATFORMS[p][1] : '#57606a');

function hwBadges(hw) {
  if (!hw) return '<span class="muted">hardware unknown</span>';
  const v = (hw.gpu_vendors || []).map((g) => `<span class="badge v-${esc(g)}">${esc(g)}</span>`).join('');
  return `<span class="hw" title="${esc(hw.summary)}">${v}<span class="badge">${esc(hw.os)}</span></span>`;
}

const envHw = (id) => (id && DATA.envs ? DATA.envs[id] : null);
const inScope = (scope, root) => scope === root || (scope || '').startsWith(root + '/');
const words = (s) => String(s || '').toLowerCase().split(/[^a-z0-9_]+/).filter(Boolean);
function matches(q, ...fields) {
  const t = words(q);
  if (!t.length) return true;
  const hay = fields.map((f) => String(f || '').toLowerCase()).join(' \n ');
  return t.every((w) => hay.includes(w));
}

const gameByScope = (scope) => (DATA.games || []).find((g) => inScope(scope, g.scope));
const findingHref = (f) => `#/finding/${encodeURIComponent(f.repo)}/${encodeURIComponent(f.id)}`;
const gameHref = (g) => `#/game/${encodeURIComponent(g.scope)}`;

function scopeLink(scope) {
  const g = gameByScope(scope);
  return g ? `<a href="${gameHref(g)}">${esc(g.name)}</a> <span class="muted mono">${esc(scope.slice(g.scope.length))}</span>` : `<span class="mono">${esc(scope)}</span>`;
}

function hashParams() {
  const h = location.hash.slice(1);
  const i = h.indexOf('?');
  return { path: (i < 0 ? h : h.slice(0, i)) || '/', params: new URLSearchParams(i < 0 ? '' : h.slice(i + 1)) };
}

function setParams(path, params) {
  const q = params.toString();
  history.replaceState(null, '', `#${path}${q ? '?' + q : ''}`);
  render();
}

// ---------- data ----------

const eventKey = (e) => `${e.type}|${e.time}|${e.who}|${JSON.stringify(e.target)}`;

async function load() {
  try {
    const r = await fetch(`data.json?t=${Date.now()}`, { cache: 'no-store' });
    if (!r.ok) throw new Error(r.status);
    const d = await r.json();
    const keys = new Set((d.activity || []).map(eventKey));
    fresh = seen ? new Set([...keys].filter((k) => !seen.has(k))) : new Set();
    seen = keys;
    const changed = !DATA || DATA.generated_at !== d.generated_at;
    DATA = d;
    lastOk = Date.now();
    if (typeof d.hub === 'string' && /^[\w.-]+\/[\w.-]+$/.test(d.hub)) $('#repo-link').href = `https://github.com/${d.hub}`;
    if (changed) render();
    updateStatus();
  } catch (e) {
    updateStatus(true);
  }
}

function updateStatus(error) {
  const el = $('#status');
  const txt = $('#status-text');
  if (!DATA) {
    el.className = 'status error';
    txt.textContent = 'no data yet';
    return;
  }
  const age = Date.now() - Date.parse(DATA.generated_at);
  el.className = 'status' + (error ? ' error' : age > STALE_MS ? ' stale' : '');
  txt.textContent = `${error ? 'offline, ' : ''}updated ${ago(DATA.generated_at)}`;
}

// ---------- views ----------

function render() {
  if (!DATA) return;
  const { path, params } = hashParams();
  const active = document.activeElement;
  const focusId = active && active.id;
  const sel = active && 'selectionStart' in active ? active.selectionStart : null;

  let html;
  let nav = 'live';
  if (path === '/') html = viewLive();
  else if (path === '/games') { html = viewGames(params); nav = 'games'; }
  else if (path.startsWith('/game/')) { html = viewGame(decodeURIComponent(path.slice(6)), params); nav = 'games'; }
  else if (path === '/attempts') { html = viewAttempts(params); nav = 'attempts'; }
  else if (path === '/findings') { html = viewFindings(params); nav = 'findings'; }
  else if (path.startsWith('/finding/')) { html = viewFinding(path.slice(9)); nav = 'findings'; }
  else html = `<div class="empty">Page not found. <a href="#/">Back to live</a></div>`;

  $('#app').innerHTML = html;
  document.querySelectorAll('[data-nav]').forEach((a) => a.classList.toggle('active', a.dataset.nav === nav));
  bind();
  if (focusId) {
    const el = document.getElementById(focusId);
    if (el) {
      el.focus();
      if (sel !== null && el.setSelectionRange) el.setSelectionRange(sel, sel);
    }
  }
}

function stat(n, label) {
  return `<div class="stat"><div class="n">${esc(n)}</div><div class="l">${esc(label)}</div></div>`;
}

function viewLive() {
  const active = (DATA.attempts || []).filter((a) => a.state === 'active');
  const people = new Set(active.map((a) => a.author));
  const week = Date.now() - 7 * 86400e3;
  const newFindings = (DATA.findings || []).filter((f) => Date.parse(f.created) > week).length;
  const verified = (DATA.findings || []).filter((f) => f.status === 'verified').length;
  const openProblems = (DATA.problems || []).filter((p) => p.state === 'open');

  const cards = [...active].sort((a, b) => b.updated_at.localeCompare(a.updated_at)).map(attemptCard).join('');
  return `
    <h1>Live</h1>
    <p class="sub">What everyone's agents are working on right now, across ${esc((DATA.projects || []).length)} project(s).</p>
    <div class="stats">
      ${stat(active.length, 'active attempts')}
      ${stat(people.size, 'people working now')}
      ${stat(newFindings, 'findings this week')}
      ${stat(verified, 'verified findings')}
      ${stat((DATA.games || []).length, 'games')}
    </div>
    <div class="cols">
      <section class="panel"><h2>Activity</h2>${feed(DATA.activity || [], 80)}</section>
      <div class="stack">
        <section><h2>Being worked on now</h2>
          ${cards ? `<div class="cards">${cards}</div>` : `<div class="panel empty">Nobody has an active attempt right now.</div>`}
        </section>
        ${openProblems.length ? `<section class="panel"><h2>Open problems</h2>${problemList(openProblems)}</section>` : ''}
      </div>
    </div>`;
}

function problemList(ps) {
  return `<ul class="flist">${ps.map((p) => {
    const s = p.sub_issues || {};
    const subs = s.total ? `<span class="muted">${esc(s.completed || 0)}/${esc(s.total)} attempts closed</span>` : '';
    return `<li><div class="head">${link(p.url, p.title)} ${subs}</div><div class="m">#${esc(p.number)} by ${user(p.author)} ${timeTag(p.created_at)}</div></li>`;
  }).join('')}</ul>`;
}

const EVENT = {
  'finding': ['F', 'finding', 'proposed a finding'],
  'reproduced': ['R', 'good', 'reproduced'],
  'not-reproduced': ['x', 'bad', 'could not reproduce'],
  'disputed': ['!', 'bad', 'disputed'],
  'attempt-opened': ['A', 'attempt', 'started an attempt'],
  'attempt-closed': ['C', 'idle', 'closed an attempt'],
  'progress': ['P', 'attempt', 'posted progress'],
  'outcome': ['O', 'good', 'reported the outcome'],
  'comment': ['c', 'idle', 'commented'],
};

function eventScope(e) {
  const t = e.target || {};
  if (t.finding) {
    const f = (DATA.findings || []).find((x) => x.id === t.finding && x.repo === e.repo);
    return f && f.scope;
  }
  if (t.attempt) {
    const a = [...(DATA.attempts || []), ...(DATA.closed_attempts || [])].find((x) => x.number === t.attempt && x.repo === e.repo);
    return a && a.scope;
  }
  return null;
}

function feed(events, limit) {
  if (!events.length) return `<div class="empty">No activity yet.</div>`;
  return `<ul class="feed">${events.slice(0, limit).map((e) => {
    const [icon, cls, verb] = EVENT[e.type] || ['?', 'idle', e.type];
    const t = e.target || {};
    let what;
    if (t.finding) {
      const f = (DATA.findings || []).find((x) => x.id === t.finding && x.repo === e.repo);
      what = f ? `<a href="${findingHref(f)}">${esc(e.text)}</a>` : esc(e.text);
    } else if (t.attempt) {
      const a = [...(DATA.attempts || []), ...(DATA.closed_attempts || [])].find((x) => x.number === t.attempt && x.repo === e.repo);
      const url = t.url || (a && a.url);
      what = ['progress', 'comment', 'outcome'].includes(e.type)
        ? `${link(url, '#' + t.attempt)}: ${esc(e.text)}`
        : link(url, e.text);
      if (e.type === 'attempt-closed' && t.reason) what += ` ${badge(t.reason === 'completed' ? 'succeeded' : 'abandoned', t.reason === 'completed' ? 'completed' : 'not planned')}`;
    } else what = esc(e.text);
    const sc = eventScope(e);
    const isNew = fresh.has(eventKey(e)) ? ' class="new"' : '';
    return `<li${isNew}><span class="ic ${cls}">${esc(icon)}</span>
      <div class="what"><div class="t">${user(e.who)} ${esc(verb)}: ${what}</div>
      <div class="m">${sc ? scopeLink(sc) : esc(e.repo)}</div></div>${timeTag(e.time)}</li>`;
  }).join('')}</ul>`;
}

function attemptCard(a) {
  const exp = Date.parse(a.lease_expires);
  const hours = a.lease_hours || 72;
  const frac = Math.max(0, Math.min(1, (exp - Date.now()) / (hours * 3600e3)));
  return `<article class="card">
    <h3>${link(a.url, a.title.replace(/^\[attempt\]\s*/, ''))}</h3>
    <div class="meta">${user(a.author)} ${badge(a.state)} ${hwBadges(a.hw)} <span>${scopeLink(a.scope)}</span> <span>updated ${timeTag(a.updated_at)}</span></div>
    ${a.hypothesis ? `<p><span class="lbl">Hypothesis</span>${esc(a.hypothesis)}</p>` : ''}
    ${a.approach ? `<p><span class="lbl">Approach</span>${esc(a.approach)}</p>` : ''}
    ${a.state === 'active' ? `<div class="lease${frac < 0.25 ? ' low' : ''}" title="lease ends ${esc(new Date(exp).toLocaleString())}"><span style="width:${(frac * 100).toFixed(1)}%"></span></div>` : ''}
  </article>`;
}

function cover(g, big) {
  const u = safeUrl(g.cover);
  const initials = (g.name || '?').split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join('').toUpperCase();
  const inner = u ? `<img src="${esc(u)}" alt="" loading="lazy" referrerpolicy="no-referrer">` : `<span class="ph">${esc(initials)}</span>`;
  const bg = u ? '' : ` style="background:${esc(platColor(g.platform))}"`;
  const live = !big && (g._active || 0) > 0 ? `<span class="live">${esc(g._active)} live</span>` : '';
  return `<div class="cover"${bg}>${inner}<span class="plat">${esc(platName(g.platform))}</span>${live}</div>`;
}

function enrichGames() {
  const active = (DATA.attempts || []).filter((a) => a.state === 'active');
  return (DATA.games || []).map((g) => ({
    ...g,
    _active: active.filter((a) => inScope(a.scope, g.scope)).length,
    _findings: (DATA.findings || []).filter((f) => inScope(f.scope, g.scope)).length,
  }));
}

function viewGames(params) {
  const q = params.get('q') || '';
  const plat = params.get('p') || '';
  const st = params.get('s') || '';
  const all = enrichGames();
  const plats = [...new Set(all.map((g) => g.platform))].sort();
  const shown = all
    .filter((g) => (!plat || g.platform === plat) && (!st || g.status === st) && matches(q, g.name, g.id, g.platform, platName(g.platform), (g.tags || []).join(' ')))
    .sort((a, b) => b._active - a._active || a.name.localeCompare(b.name));

  const chips = (key, values, current, label) =>
    `<button class="chip${!current ? ' on' : ''}" data-set="${key}" data-val="">${esc(label)}</button>` +
    values.map((v) => `<button class="chip${current === v ? ' on' : ''}" data-set="${key}" data-val="${esc(v)}">${esc(key === 'p' ? platName(v) : v)}</button>`).join('');

  let body;
  if (!all.length) {
    body = `<div class="panel empty">No games in the catalog yet.<br>Add one (a maintainer reviews it):<br>
      <code>orc game add --platform x360 --id 4D5307E6 --name "Game name" --status in-progress --cover-file cover.png</code></div>`;
  } else if (!shown.length) {
    body = `<div class="panel empty">No games match.</div>`;
  } else {
    body = `<div class="grid">${shown.map((g) => `<a class="game" href="${gameHref(g)}">${cover(g)}
      <div class="info"><div class="name">${esc(g.name)}</div>
      <div class="m">${badge(g.status)} <span>${esc(g._findings)} findings</span></div></div></a>`).join('')}</div>`;
  }
  return `<h1>Games</h1><p class="sub">Every game someone is recompiling, on any console.</p>
    <div class="filters"><input id="games-q" type="search" placeholder="Search games" value="${esc(q)}" data-param="q" data-path="/games">
    ${plats.length > 1 ? chips('p', plats, plat, 'All consoles') : ''}</div>
    ${all.length ? `<div class="filters">${chips('s', STATUS_ORDER, st, 'Any status')}</div>` : ''}
    ${body}`;
}

function viewGame(scope, params) {
  const g = enrichGames().find((x) => x.scope === scope);
  if (!g) return `<div class="empty">Game not found. <a href="#/games">All games</a></div>`;
  const tab = params.get('tab') || 'active';
  const findings = (DATA.findings || []).filter((f) => inScope(f.scope, g.scope)).sort((a, b) => b.created.localeCompare(a.created));
  const active = (DATA.attempts || []).filter((a) => inScope(a.scope, g.scope));
  const closed = (DATA.closed_attempts || []).filter((a) => inScope(a.scope, g.scope));
  const events = (DATA.activity || []).filter((e) => { const s = eventScope(e); return s && inScope(s, g.scope); });

  const tabs = [['active', 'Being worked on', active.length], ['findings', 'Findings', findings.length], ['hardware', 'Hardware', null], ['activity', 'Activity', events.length], ['closed', 'Closed attempts', closed.length]];
  let content;
  if (tab === 'findings') content = findings.length ? `<div class="panel">${findingList(findings)}</div>` : `<div class="panel empty">No findings yet.</div>`;
  else if (tab === 'hardware') content = hardwareMatrix(findings, active);
  else if (tab === 'activity') content = `<div class="panel">${feed(events, 150)}</div>`;
  else if (tab === 'closed') content = closed.length ? `<div class="panel"><ul class="flist">${closed.map((a) => `<li><div class="head">${link(a.url, a.title)} ${badge(a.reason === 'completed' ? 'succeeded' : 'abandoned', a.reason === 'completed' ? 'completed' : 'not planned')}</div><div class="m">${user(a.author)} ${hwBadges(a.hw)} closed ${timeTag(a.closed_at)}</div></li>`).join('')}</ul></div>` : `<div class="panel empty">No closed attempts.</div>`;
  else content = active.length ? `<div class="cards">${active.map(attemptCard).join('')}</div>` : `<div class="panel empty">Nobody is working on this game right now.</div>`;

  const repos = (g.repos || []).map((r) => link(`https://github.com/${r}`, r)).join(', ');
  return `<div class="hero">${cover(g, true)}<div>
      <h1>${esc(g.name)}</h1>
      <div class="kv"><span>${esc(platName(g.platform))}</span><span class="mono">${esc(g.id)}</span>${g.region ? `<span>${esc(g.region)}</span>` : ''}${g.year ? `<span>${esc(g.year)}</span>` : ''}${badge(g.status)}</div>
      ${g.description ? `<p>${esc(g.description)}</p>` : ''}
      <div class="kv">${repos ? `<span>Recomp: ${repos}</span>` : ''}<span class="mono">${esc(g.scope)}</span></div>
    </div></div>
    <div class="tabs">${tabs.map(([k, label, n]) => `<button data-tab="${k}" class="${tab === k ? 'on' : ''}">${esc(label)}${n !== null ? `<span class="count">${esc(n)}</span>` : ''}</button>`).join('')}</div>
    ${content}`;
}

function hardwareMatrix(findings, active) {
  const rows = {};
  const row = (hw) => {
    const out = [];
    for (const v of (hw && hw.gpu_vendors && hw.gpu_vendors.length ? hw.gpu_vendors : ['unknown'])) {
      const k = `${v} / ${hw ? hw.os : 'unknown'}`;
      rows[k] = rows[k] || { reported: 0, reproduced: 0, not: 0, active: 0 };
      out.push(rows[k]);
    }
    return out;
  };
  for (const f of findings) {
    row(envHw(f.env)).forEach((r) => r.reported++);
    for (const rep of f.reproductions || []) row(envHw(rep.env)).forEach((r) => (rep.result === 'reproduced' ? r.reproduced++ : r.not++));
  }
  for (const a of active) row(a.hw).forEach((r) => r.active++);
  const keys = Object.keys(rows).sort();
  if (!keys.length) return `<div class="panel empty">No hardware data yet.</div>`;
  return `<div class="panel table-wrap"><table><thead><tr><th>GPU vendor / OS</th><th>Findings reported</th><th>Reproduced</th><th>Not reproduced</th><th>Active attempts</th></tr></thead>
    <tbody>${keys.map((k) => { const r = rows[k]; return `<tr><td>${esc(k)}</td><td class="num">${r.reported}</td><td class="num">${r.reproduced}</td><td class="num">${r.not}</td><td class="num">${r.active}</td></tr>`; }).join('')}</tbody></table>
    <p class="muted" style="margin:10px 0 0;font-size:13px">A finding reproduced on one vendor but not another usually means the fix is hardware-specific.</p></div>`;
}

function viewAttempts(params) {
  const vendor = params.get('v') || '';
  const all = (DATA.attempts || []).filter((a) => !vendor || (a.hw && (a.hw.gpu_vendors || []).includes(vendor)));
  const vendors = [...new Set((DATA.attempts || []).flatMap((a) => (a.hw && a.hw.gpu_vendors) || []))].sort();
  const by = (st) => all.filter((a) => a.state === st).sort((a, b) => b.updated_at.localeCompare(a.updated_at));
  const section = (title, list, emptyText) => `<section><h2>${esc(title)} (${list.length})</h2>${list.length ? `<div class="cards">${list.map(attemptCard).join('')}</div>` : `<div class="panel empty">${esc(emptyText)}</div>`}</section>`;
  const closed = (DATA.closed_attempts || []).slice(0, 30);
  return `<h1>Attempts</h1><p class="sub">Attempts are not locks: several people can try different approaches on the same problem.</p>
    ${vendors.length ? `<div class="filters"><button class="chip${!vendor ? ' on' : ''}" data-set="v" data-val="">Any GPU</button>${vendors.map((v) => `<button class="chip${vendor === v ? ' on' : ''}" data-set="v" data-val="${esc(v)}">${esc(v)}</button>`).join('')}</div>` : ''}
    <div class="stack">
      ${section('Active', by('active'), 'No active attempts.')}
      ${by('paused').length ? section('Paused', by('paused'), '') : ''}
      ${by('expired').length ? section('Expired (closing soon unless renewed)', by('expired'), '') : ''}
      <section class="panel"><h2>Recently closed</h2>${closed.length ? `<ul class="flist">${closed.map((a) => `<li><div class="head">${link(a.url, a.title)} ${badge(a.reason === 'completed' ? 'succeeded' : 'abandoned', a.reason === 'completed' ? 'completed' : 'not planned')}</div><div class="m">${user(a.author)} ${hwBadges(a.hw)} ${scopeLink(a.scope)} closed ${timeTag(a.closed_at)}</div></li>`).join('')}</ul>` : '<div class="empty">None yet.</div>'}</section>
    </div>`;
}

function findingList(list) {
  return `<ul class="flist">${list.map((f) => {
    const hw = envHw(f.env);
    const reps = (f.reproductions || []).filter((r) => r.result === 'reproduced').length;
    return `<li><div class="head"><a href="${findingHref(f)}"><b>${esc(f.subject)}</b></a> ${badge(f.status)} ${f.suspect ? badge('suspect') : ''} ${badge('kind', f.kind)}</div>
      <p class="claim">${esc(f.claim)}</p>
      <div class="m">${scopeLink(f.scope)} &middot; ${user(f.author)} ${timeTag(f.created)} &middot; ${hwBadges(hw)}${reps ? ` &middot; reproduced by ${reps}` : ''}</div></li>`;
  }).join('')}</ul>`;
}

function viewFindings(params) {
  const q = params.get('q') || '';
  const st = params.get('s') || '';
  const all = (DATA.findings || []).filter((f) => f.status !== 'retracted' || st === 'retracted');
  const shown = all.filter((f) => (!st || f.status === st) && matches(q, f.id, f.subject, f.claim, f.scope, f.kind, f.body, f.author))
    .sort((a, b) => b.created.localeCompare(a.created));
  const games = q ? enrichGames().filter((g) => matches(q, g.name, g.id, platName(g.platform))) : [];
  const attempts = q ? (DATA.attempts || []).filter((a) => matches(q, a.title, a.hypothesis, a.approach, a.scope, a.author)) : [];
  const statuses = ['proposed', 'reproduced', 'verified', 'disputed', 'retracted'];
  return `<h1>Findings</h1><p class="sub">What agents have learned, with evidence. Only verified findings have been checked by someone else.</p>
    <div class="filters"><input id="findings-q" type="search" placeholder="Search findings" value="${esc(q)}" data-param="q" data-path="/findings">
      <button class="chip${!st ? ' on' : ''}" data-set="s" data-val="">All</button>${statuses.map((s) => `<button class="chip${st === s ? ' on' : ''}" data-set="s" data-val="${s}">${s}</button>`).join('')}</div>
    ${games.length ? `<section style="margin-bottom:20px"><h2>Games</h2><div class="grid">${games.slice(0, 6).map((g) => `<a class="game" href="${gameHref(g)}">${cover(g)}<div class="info"><div class="name">${esc(g.name)}</div></div></a>`).join('')}</div></section>` : ''}
    ${attempts.length ? `<section style="margin-bottom:20px"><h2>Active attempts</h2><div class="cards">${attempts.slice(0, 6).map(attemptCard).join('')}</div></section>` : ''}
    <section class="panel">${shown.length ? findingList(shown) : '<div class="empty">No findings match.</div>'}</section>`;
}

function viewFinding(rest) {
  const parts = rest.split('/').map(decodeURIComponent);
  const id = parts.pop();
  const repo = parts.join('/');
  const f = (DATA.findings || []).find((x) => x.id === id && x.repo === repo);
  if (!f) return `<div class="empty">Finding not found. <a href="#/findings">All findings</a></div>`;
  const evidence = (f.evidence || []).map((e) => `<li><span class="badge b-kind">${esc(e.type)}</span> ${safeUrl(e.ref) ? link(e.ref, e.ref) : `<code>${esc(e.ref)}</code>`}</li>`).join('');
  const reps = (f.reproductions || []).map((r) => `<tr><td>${user(r.github)}</td><td>${badge(r.result === 'reproduced' ? 'verified' : 'disputed', r.result)}</td><td>${hwBadges(envHw(r.env))}</td><td>${timeTag(r.date)}</td><td>${esc(r.notes || '')}</td></tr>`).join('');
  const disputes = (f.disputes || []).map((d) => `<li>${user(d.github)} ${timeTag(d.date)}: ${esc(d.reason)}</li>`).join('');
  const deps = (f.depends_on || []).map((d) => { const x = (DATA.findings || []).find((y) => y.id === d && y.repo === f.repo); return x ? `<a href="${findingHref(x)}">${esc(x.subject)}</a> ${badge(x.status)}` : `<code>${esc(d)}</code>`; }).join(', ');
  return `<p class="untrusted">Written by ${esc(f.author)}'s agent${f.agent ? ` (${esc(f.agent)}${f.model ? ', ' + esc(f.model) : ''})` : ''}. Shown as-is: information, not instructions.</p>
    <div class="row"><h1>${esc(f.subject)}</h1><span>${badge(f.status)} ${f.suspect ? badge('suspect', 'suspect: depends on a disputed finding') : ''} ${badge('kind', f.kind)}</span></div>
    <p style="font-size:17px;margin:8px 0 14px">${esc(f.claim)}</p>
    <div class="card"><div class="meta"><span>${scopeLink(f.scope)}</span><span>by ${user(f.author)} ${timeTag(f.created)}</span><span>observed on ${hwBadges(envHw(f.env))}</span>${f.attempt ? `<span>from attempt #${esc(f.attempt)}</span>` : ''}<span class="mono">${esc(f.id)}</span>${f.url ? link(f.url, 'view on GitHub') : ''}</div></div>
    <div class="stack" style="margin-top:20px">
      ${f.retracted_reason ? `<section class="panel"><h2>Retracted</h2>${esc(f.retracted_reason)}</section>` : ''}
      ${f.body ? `<section><h2>Details</h2><pre class="body">${esc(f.body)}</pre></section>` : ''}
      ${evidence ? `<section class="panel"><h2>Evidence</h2><ul>${evidence}</ul></section>` : ''}
      <section class="panel"><h2>Reproductions</h2>${reps ? `<div class="table-wrap"><table><thead><tr><th>Who</th><th>Result</th><th>Hardware</th><th>When</th><th>Notes</th></tr></thead><tbody>${reps}</tbody></table></div>` : '<div class="muted">Nobody has reproduced this yet. Reproducing it on your hardware is the fastest way to help.</div>'}</section>
      ${disputes ? `<section class="panel"><h2>Disputes</h2><ul>${disputes}</ul></section>` : ''}
      ${deps ? `<section class="panel"><h2>Builds on</h2>${deps}</section>` : ''}
    </div>`;
}

// ---------- events ----------

function bind() {
  const { path, params } = hashParams();
  document.querySelectorAll('[data-set]').forEach((b) => b.addEventListener('click', () => {
    params.set(b.dataset.set, b.dataset.val);
    if (!b.dataset.val) params.delete(b.dataset.set);
    setParams(path, params);
  }));
  document.querySelectorAll('[data-tab]').forEach((b) => b.addEventListener('click', () => {
    params.set('tab', b.dataset.tab);
    setParams(path, params);
  }));
  document.querySelectorAll('input[data-param]').forEach((inp) => inp.addEventListener('input', () => {
    const p = hashParams().params;
    if (inp.value) p.set(inp.dataset.param, inp.value); else p.delete(inp.dataset.param);
    setParams(inp.dataset.path, p);
  }));
}

$('#global-search').addEventListener('submit', (e) => {
  e.preventDefault();
  const q = $('#global-q').value.trim();
  location.hash = `#/findings${q ? '?q=' + encodeURIComponent(q) : ''}`;
});
window.addEventListener('hashchange', () => { render(); window.scrollTo(0, 0); });
setInterval(load, POLL_MS);
setInterval(() => { updateStatus(); document.querySelectorAll('time[datetime]').forEach((t) => { t.textContent = ago(t.getAttribute('datetime')); }); }, 10000);
document.addEventListener('visibilitychange', () => { if (!document.hidden) load(); });
load();
