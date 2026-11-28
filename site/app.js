(function () {
  var L = window.I18N || {};
  var sel = document.getElementById('lang');
  function pick() {
    try { var s = localStorage.getItem('lang'); if (s && L[s]) return s; } catch (e) {}
    var want = (navigator.languages || [navigator.language || 'en']);
    for (var i = 0; i < want.length; i++) {
      var c = String(want[i]).slice(0, 2).toLowerCase();
      if (L[c]) return c;
    }
    return 'en';
  }
  function apply(code) {
    var d = L[code]; if (!d) return;
    document.documentElement.lang = code;
    var els = document.querySelectorAll('[data-i]');
    for (var i = 0; i < els.length; i++) {
      var k = els[i].getAttribute('data-i');
      if (d[k] !== undefined) els[i].innerHTML = d[k];
    }
    var caps = document.querySelectorAll('img[data-alt]');
    for (var j = 0; j < caps.length; j++) {
      var a = d[caps[j].getAttribute('data-alt')];
      if (a !== undefined) caps[j].alt = a.replace(/<[^>]*>/g, '');
    }
    sel.value = code;
  }
  sel.addEventListener('change', function () {
    try { localStorage.setItem('lang', sel.value); } catch (e) {}
    apply(sel.value);
    board.lang();
  });

  // Classement mondial (/api/scores du worker), charge quand la section devient visible
  var board = (function () {
    var lb = document.getElementById('lb');
    if (!lb) return { lang: function () {} };
    function $(id) { return document.getElementById(id); }
    var map = $('lb-map'), mode = $('lb-mode'), ver = $('lb-ver'), msg = $('lb-msg'), body = lb.tBodies[0];
    var data = null, status = 'idle';
    function T() { return (L[sel.value] || L.en).board; }
    function chars(id) {
      return String(id).split(',').map(function (c) { return T().chars[c] || L.en.board.chars[c] || c; }).join(' + ');
    }
    function time(t) { t = Math.floor(t); return Math.floor(t / 60) + ':' + ('0' + t % 60).slice(-2); }
    function cell(tr, text, cls) { var td = tr.insertCell(); td.textContent = text; if (cls) td.className = cls; return td; }
    function options(s, items, keep) {
      var v = s.value; s.textContent = '';
      items.forEach(function (it) { var o = document.createElement('option'); o.value = it[0]; o.textContent = it[1]; s.appendChild(o); });
      if (keep && v) s.value = v;
      if (s.selectedIndex < 0) s.selectedIndex = 0;
    }
    function render() {
      var t = T();
      $('lb-l-stage').textContent = t.stage; $('lb-l-mode').textContent = t.mode; $('lb-l-ver').textContent = t.version;
      ['player', 'char', 'score', 'time', 'kills', 'diff'].forEach(function (k) { $('lb-h-' + k).textContent = t[k]; });
      $('lb-legend').textContent = t.legend;
      options(map, Object.keys(t.maps).map(function (k) { return [k, t.maps[k]]; }), true);
      var modes = [['1', t.solo]];
      for (var n = 2; n <= 4; n++) modes.push([String(n), t.coop.replace('{n}', n)]);
      options(mode, modes, true);
      body.textContent = '';
      msg.textContent = status === 'loading' ? t.loading : status === 'failed' ? t.failed : '';
      if (!data) return;
      options(ver, data.versions.map(function (v) { return [v, v]; }), false);
      ver.value = data.version;
      if (!data.scores.length) msg.textContent = t.empty;
      data.scores.forEach(function (e) {
        var tr = body.insertRow();
        tr.className = e.won ? 'won' : 'lost';
        cell(tr, e.rank);
        cell(tr, e.name, e.title ? 't' + Math.min(e.title, 3) : '');
        cell(tr, chars(e.character));
        cell(tr, e.score.toLocaleString(sel.value), 'num');
        cell(tr, time(e.time), 'num');
        cell(tr, e.kills.toLocaleString(sel.value), 'num');
        cell(tr, t.diffs[e.difficulty] || '');
      });
    }
    function load(version) {
      status = 'loading'; data = null; render();
      var q = 'map=' + encodeURIComponent(map.value) + '&players=' + mode.value + '&version=' + encodeURIComponent(version || '');
      fetch('/api/scores?' + q).then(function (r) { if (!r.ok) throw r; return r.json(); })
        .then(function (d) { data = d; status = 'done'; render(); })
        .catch(function () { status = 'failed'; render(); });
    }
    map.addEventListener('change', function () { load(''); });
    mode.addEventListener('change', function () { load(ver.value); });
    ver.addEventListener('change', function () { load(ver.value); });
    render();
    var started = false;
    function start() { if (!started) { started = true; load(''); } }
    if ('IntersectionObserver' in window) {
      new IntersectionObserver(function (es, o) { if (es[0].isIntersecting) { o.disconnect(); start(); } }).observe(lb);
    } else start();
    return { lang: render };
  })();
  apply(pick());
  board.lang();
})();
