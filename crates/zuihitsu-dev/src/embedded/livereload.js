// zuihitsu-dev — browser-side livereload + CSS HMR + error overlay.
// Auto-injected into every HTML response by the dev server.
(function () {
  function connect() {
    var proto = location.protocol === 'https:' ? 'wss://' : 'ws://';
    var ws = new WebSocket(proto + location.host + '/__dev/ws');

    ws.addEventListener('open', function () {
      console.log('[zuihitsu-dev] connected');
      hideOverlay();
    });
    ws.addEventListener('close', function () {
      console.log('[zuihitsu-dev] disconnected; retrying in 1s');
      setTimeout(connect, 1000);
    });
    ws.addEventListener('error', function () {
      // close handler will fire next.
    });
    ws.addEventListener('message', function (e) {
      var msg;
      try { msg = JSON.parse(e.data); } catch (_) { return; }
      switch (msg.type) {
        case 'css':    swapCss(msg.path); break;
        case 'reload': location.reload(); break;
        case 'error':  showOverlay(msg.body || ''); break;
        case 'ok':     hideOverlay(); break;
      }
    });
  }

  function swapCss(path) {
    var links = document.querySelectorAll('link[rel="stylesheet"]');
    var swapped = 0;
    links.forEach(function (link) {
      try {
        var u = new URL(link.href, location.origin);
        if (u.pathname === path) {
          u.searchParams.set('__v', Date.now().toString());
          link.href = u.toString();
          swapped++;
        }
      } catch (_) {}
    });
    console.log('[zuihitsu-dev] css ' + path + ' swapped ' + swapped);
    if (swapped === 0) location.reload();
  }

  function showOverlay(body) {
    var el = document.getElementById('__zuihitsu_dev_err');
    if (!el) {
      el = document.createElement('div');
      el.id = '__zuihitsu_dev_err';
      el.style.cssText = [
        'position:fixed', 'inset:0',
        'background:rgba(46,52,64,0.95)', 'color:#eceff4',
        'font:13px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace',
        'padding:24px 28px', 'overflow:auto', 'z-index:99999',
        'white-space:pre-wrap', 'word-break:break-word'
      ].join(';');
      document.body.appendChild(el);
    }
    el.innerHTML = '';
    var head = document.createElement('div');
    head.textContent = 'zuihitsu-dev — build error';
    head.style.cssText = 'color:#bf616a;margin-bottom:12px;font-weight:bold;';
    el.appendChild(head);
    var pre = document.createElement('div');
    pre.textContent = body;
    el.appendChild(pre);
  }

  function hideOverlay() {
    var el = document.getElementById('__zuihitsu_dev_err');
    if (el) el.remove();
  }

  connect();
})();
