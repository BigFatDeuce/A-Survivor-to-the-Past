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
  });
  apply(pick());
})();
