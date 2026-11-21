const byId = (id) => document.getElementById(id);
const drawing = byId('drawing');
const play = byId('play');
const progress = byId('progress');
const speed = byId('speed');
const preference = matchMedia('(prefers-reduced-motion: reduce)');
let samples;
let svg;
let animations = [];
let clock;
let durationMs = 0;
let playing = false;
let frame = 0;
let exportSvg = '';
let revision = 0;

function currentTime() {
  return Math.max(0, Math.min(durationMs, Number(clock?.currentTime ?? durationMs)));
}

function update() {
  const time = currentTime();
  progress.value = String(durationMs ? Math.round((time / durationMs) * 1000) : 1000);
  progress.setAttribute(
    'aria-valuetext',
    `${(time / 1000).toFixed(1)} of ${(durationMs / 1000).toFixed(1)} seconds`
  );
  byId('time').textContent = `${(time / 1000).toFixed(1)} / ${(durationMs / 1000).toFixed(1)} s`;
  play.textContent = playing ? 'Pause' : time >= durationMs ? 'Play again' : 'Play';
}

function pause() {
  const wasPlaying = playing;
  playing = false;
  cancelAnimationFrame(frame);
  animations.forEach((animation) => animation.pause());
  update();
  if (wasPlaying && currentTime() < durationMs) {
    byId('status').textContent = 'Paused. Drag the playhead or press Play to continue.';
  }
}

function seek(time) {
  // Clear sub-millisecond rounding at the final animation boundary.
  const target = time >= durationMs ? durationMs + 1 : time;
  animations.forEach((animation) => {
    animation.currentTime = target;
  });
  update();
}

function tick() {
  if (!playing) return;
  if (currentTime() >= durationMs) {
    pause();
    seek(durationMs);
    byId('status').textContent = 'Complete. Replay, explore another order, or download the SVG.';
    return;
  }
  update();
  frame = requestAnimationFrame(tick);
}

function start(fromBeginning = false) {
  if (!animations.length || preference.matches) return;
  pause();
  if (fromBeginning || currentTime() >= durationMs) seek(0);
  // Give every CSS animation the same clock, including animations with delayed starts.
  const time = currentTime();
  const startTime = document.timeline.currentTime - time;
  animations.forEach((animation) => {
    animation.play();
    animation.startTime = startTime;
  });
  playing = true;
  byId('status').textContent = 'Drawing the diagram. Drag the playhead to inspect a moment.';
  tick();
}

async function render({ autoplay = false } = {}) {
  pause();
  const currentRevision = ++revision;
  for (const id of ['play', 'replay', 'finish', 'progress', 'download']) byId(id).disabled = true;
  const sample = samples.find((candidate) => candidate.id === byId('sample').value);
  try {
    const response = await fetch(
      `/render?${new URLSearchParams({ sample: sample.id, speed: speed.value, order: byId('order').value })}`
    );
    const markup = await response.text();
    if (currentRevision !== revision) return;
    if (!response.ok) throw new Error(markup);
    byId('error').hidden = true;
    byId('speed-value').textContent = `${Number(speed.value)}×`;
    byId('diagram-title').textContent = sample.title;
    byId('diagram-description').textContent = sample.description;
    byId('source').textContent = sample.source;
    byId('order-note').textContent =
      byId('order').value === 'flow'
        ? 'Branches or independent chart elements draw together, each at the selected speed.'
        : 'Connections, bars, or sectors draw one at a time, each with a soft start and finish.';
    byId('order').querySelector('[value="flow"]').textContent = [
      'request',
      'review',
      'styles',
      'gitgraph',
      'mindmap',
    ].includes(sample.id)
      ? 'Follow the flow'
      : 'Parallel elements';
    // Native ferrigura output from the local preview server.
    const documentSvg = new DOMParser().parseFromString(markup, 'image/svg+xml');
    svg = document.importNode(documentSvg.documentElement, true);
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', sample.description);
    drawing.replaceChildren(svg);
    durationMs = Number(
      svg.querySelector('[data-draw-duration]').getAttribute('data-draw-duration')
    );
    exportSvg = markup;
    animations = svg.getAnimations({ subtree: true });
    clock = animations.reduce(
      (last, animation) =>
        !last ||
        animation.effect.getComputedTiming().endTime > last.effect.getComputedTiming().endTime
          ? animation
          : last,
      undefined
    );
    pause();
    byId('download').disabled = false;
    for (const id of ['play', 'replay', 'finish', 'progress'])
      byId(id).disabled = preference.matches;
    byId('motion-note').hidden = !preference.matches;
    if (preference.matches) {
      byId('status').textContent = 'Reduced motion: the complete diagram is shown.';
      seek(durationMs);
    } else if (autoplay) {
      start(true);
    } else {
      seek(durationMs);
      byId('status').textContent = 'Ready. Play the diagram or drag the playhead.';
    }
  } catch (error) {
    if (currentRevision !== revision) return;
    byId('error').hidden = false;
    byId('error').textContent = error.message;
    byId('status').textContent = 'The preview could not be loaded.';
  }
}

play.addEventListener('click', () => (playing ? pause() : start()));
byId('replay').addEventListener('click', () => start(true));
byId('finish').addEventListener('click', () => {
  pause();
  seek(durationMs);
  byId('status').textContent = 'The complete diagram is shown.';
});
progress.addEventListener('input', () => {
  const time = (Number(progress.value) / 1000) * durationMs;
  pause();
  seek(time);
  byId('status').textContent =
    time >= durationMs
      ? 'The complete diagram is shown.'
      : 'Paused. Drag the playhead or press Play to continue.';
});
for (const id of ['sample', 'order', 'speed']) {
  byId(id).addEventListener('change', () => render({ autoplay: !preference.matches }));
}
speed.addEventListener('input', () => {
  byId('speed-value').textContent = `${Number(speed.value)}×`;
});
preference.addEventListener('change', () => render());
document.addEventListener('visibilitychange', () => {
  if (document.hidden) pause();
});
const observer = new IntersectionObserver(([entry]) => {
  if (!entry.isIntersecting) pause();
});
observer.observe(drawing);
byId('download').addEventListener('click', () => {
  const url = URL.createObjectURL(new Blob([exportSvg], { type: 'image/svg+xml' }));
  const link = document.createElement('a');
  link.href = url;
  link.download = `ferrigura-${byId('sample').value}-${byId('order').value}-animated.svg`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
});
window.addEventListener('pagehide', () => {
  pause();
  observer.disconnect();
});

try {
  const response = await fetch('/samples.json');
  if (!response.ok) throw new Error('The diagram samples could not be loaded.');
  ({ samples } = await response.json());
  await render();
} catch (error) {
  byId('error').hidden = false;
  byId('error').textContent = error.message;
  byId('status').textContent = 'The preview could not be loaded.';
}
