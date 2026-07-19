(() => {
  // Placeholder maplibre bundle for environments where the real MapLibre runtime is unavailable.
  if (typeof window === 'undefined') {
    return;
  }

  window.maplibregl = window.maplibregl || {
    Map: class {
      constructor() {
        throw new Error('maplibre-gl.js placeholder: map constructor is disabled in this example harness');
      }
    },
  };
})();
