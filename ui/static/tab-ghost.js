// Floating copy of a dragged tab (ADR-027): name and flag come from the query string.
const q = new URLSearchParams(location.search);
if (q.get('d') === '1') document.documentElement.classList.add('dark');
const flag = q.get('f');
if (flag && /^[a-z-]{2,12}$/.test(flag)) {
  const img = document.createElement('img');
  img.src = '/flags/' + flag + '.svg';
  img.alt = '';
  img.onerror = () => img.remove();
  document.body.append(img);
}
const name = document.createElement('span');
name.textContent = q.get('t') ?? '';
document.body.append(name);
