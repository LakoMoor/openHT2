const figures = [...document.querySelectorAll('.shots figure')];
const viewer = document.querySelector('#viewer');
const photo = document.querySelector('#viewer-image');
let visible = figures.slice();
let current = 0;
let opener;
function show(index) {
  current = (index + visible.length) % visible.length;
  const figure = visible[current];
  const source = figure.querySelector('img');
  photo.src = figure.querySelector('a').href;
  photo.alt = source.alt;
  document.querySelector('#viewer-caption').textContent = figure.querySelector('figcaption').textContent;
  document.querySelector('#viewer-counter').textContent = `${current + 1} / ${visible.length}`;
}
function close() { viewer.close(); }
viewer.addEventListener('close', () => { document.body.style.overflow = ''; opener?.focus(); });
document.querySelector('#viewer-close').addEventListener('click', close);
document.querySelector('#viewer-prev').addEventListener('click', () => show(current - 1));
document.querySelector('#viewer-next').addEventListener('click', () => show(current + 1));
viewer.addEventListener('click', event => { if (event.target === viewer) { const r=viewer.getBoundingClientRect(); if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom) close(); } });
viewer.addEventListener('keydown', event => {
  if (event.key === 'ArrowLeft') { event.preventDefault(); show(current - 1); }
  if (event.key === 'ArrowRight') { event.preventDefault(); show(current + 1); }
});
for (const figure of figures) figure.querySelector('a').addEventListener('click', event => {
  if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey || !viewer.showModal) return;
  event.preventDefault(); opener = event.currentTarget;
  show(visible.indexOf(figure)); viewer.showModal(); document.body.style.overflow = 'hidden';
});
function count() { document.querySelector('#gallery-count').textContent = `${visible.length} кадров · новые снимки: 26 октября 2026`; }
for (const button of document.querySelectorAll('[data-filter]')) button.addEventListener('click', () => {
  document.querySelectorAll('[data-filter]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
  for (const figure of figures) figure.hidden = button.dataset.filter !== 'all' && figure.dataset.category !== button.dataset.filter;
  visible = figures.filter(figure => !figure.hidden); count();
});
count();
