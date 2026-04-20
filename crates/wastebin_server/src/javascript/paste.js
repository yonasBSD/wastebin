function $(id) {
  return document.getElementById(id);
}

document.addEventListener('keydown', onKey);
$("copy-button").addEventListener("click", copy);

function highlightLines(scroll) {
  document.querySelectorAll('tr.line-highlight').forEach(tr => {
    tr.classList.remove('line-highlight');
  });

  const match = window.location.hash.match(/^#L(\d+)(?:-L(\d+))?$/);
  if (!match) return;

  const a = parseInt(match[1], 10);
  const b = match[2] ? parseInt(match[2], 10) : a;
  const from = Math.min(a, b);
  const to = Math.max(a, b);

  for (let i = from; i <= to; i++) {
    const td = document.getElementById('L' + i);
    if (td && td.parentElement) {
      td.parentElement.classList.add('line-highlight');
    }
  }

  if (scroll && match[2]) {
    const firstTd = document.getElementById('L' + from);
    if (firstTd) firstTd.scrollIntoView({ block: 'center' });
  }
}

window.addEventListener('hashchange', () => highlightLines(true));
highlightLines(true);

document.querySelectorAll('td.line-number > a').forEach(a => {
  a.addEventListener('click', (e) => {
    if (!e.shiftKey) return;
    const m = a.getAttribute('href').match(/^#L(\d+)$/);
    const current = window.location.hash.match(/^#L(\d+)(?:-L\d+)?$/);
    if (!m || !current) return;
    e.preventDefault();
    const clicked = parseInt(m[1], 10);
    const base = parseInt(current[1], 10);
    const from = Math.min(base, clicked);
    const to = Math.max(base, clicked);
    history.replaceState(null, '', from === to ? '#L' + from : '#L' + from + '-L' + to);
    highlightLines(false);
  });
});

function showToast(text, timeout) {
  let toast = $("toast");

  toast.innerText = text;
  toast.classList.toggle("hidden");
  toast.classList.toggle("shown");

  setTimeout(() => {
    toast.classList.toggle("hidden");
    toast.classList.toggle("shown");
  }, timeout);
}

function copy() {
  const lines = document.querySelectorAll('td.line');
  const content = Array.from(lines)
    .map(line => line.textContent)
    .join('')
    .trim();

  navigator.clipboard.writeText(content)
    .then(() => {
      showToast("Copied content", 1500);
    }, function(err) {
      console.error("failed to copy content", err);
    });
}

function onKey(e) {
  if (e.key == 'n') {
    window.location.href = "/";
  }
  else if (e.key == 'r') {
    window.location.href = "/raw" + window.location.pathname;
  }
  else if (e.key == 'y') {
    navigator.clipboard.writeText(window.location.href);
    showToast("Copied URL", 1500);
  }
  else if (e.key == 'd') {
    window.location.href = "/dl" + window.location.pathname;
  }
  else if (e.key == 'q') {
    window.location.href = "/qr" + window.location.pathname;
  }
  else if (e.key == 'p') {
    window.location.href = window.location.href.split("?")[0];
  }
  else if (e.key == 'c' && !(e.ctrlKey || e.metaKey)) {
    copy();
  }
  else if (e.key == 'w') {
    document.body.classList.toggle('line-wrap');
  }
  else if (e.key == 'm') {
    const toggle = document.getElementById('view-toggle');
    if (toggle) window.location.href = toggle.href;
  }
  else if (e.key == '?') {
    var overlay = document.getElementById("overlay");

    overlay.style.display = overlay.style.display != "block" ? "block" : "none";
    overlay.onclick = function() {
      if (overlay.style.display == "block") {
        overlay.style.display = "none";
      }
    };
  }

  if (e.keyCode == 27) {
    var overlay = document.getElementById("overlay");

    if (overlay.style.display == "block") {
      overlay.style.display = "none";
    }
  }
}
