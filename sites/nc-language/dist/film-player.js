export async function prepareFilmVideo(video) {
  const response = await fetch(video.querySelector('source').src);
  if (!response.ok) throw new Error('Video request failed');
  const blob = await response.blob();
  if (!blob.size || !blob.type.startsWith('video/')) throw new Error('Invalid video response');
  const objectURL = URL.createObjectURL(blob);
  video.src = objectURL;
  return objectURL;
}

export function initFilmPlayer() {
  const video = document.querySelector('#nc-overview-video');
  const button = document.querySelector('#film-play');
  const status = document.querySelector('#film-play-status');
  if (!video || !button || !status) return;
  let objectURL;
  video.controls = false;
  button.hidden = false;
  video.addEventListener('playing', () => { status.hidden = true; });
  button.addEventListener('click', async () => {
    button.disabled = true;
    status.hidden = false;
    status.textContent = 'Loading the film…';
    try {
      // The static host ignores Range requests; a local blob allows reliable seeking.
      objectURL = await prepareFilmVideo(video);
    } catch {
      video.controls = true;
      button.hidden = true;
      status.textContent = 'Use the video controls or download the film to watch it.';
      return;
    }
    video.controls = true;
    button.hidden = true;
    try {
      await video.play();
      status.hidden = true;
    } catch {
      status.textContent = 'Video ready. Press play to start.';
    }
  });
  addEventListener('pagehide', event => {
    if (!event.persisted && objectURL) URL.revokeObjectURL(objectURL);
  });
}
