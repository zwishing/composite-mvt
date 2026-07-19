(() => {
  'use strict';

  const statusElement = document.getElementById('status');
  if (!statusElement) {
    throw new Error('missing status panel');
  }

  let state = 'loading';

  const setTerminalState = (next, roads = 0, buildings = 0) => {
    if (state !== 'loading') {
      return;
    }
    state = next;
    statusElement.dataset.state = next;
    statusElement.dataset.roads = String(roads);
    statusElement.dataset.buildings = String(buildings);
    statusElement.textContent = next;
  };

  const map = new maplibregl.Map({
    container: 'map',
    style: { version: 8, sources: {}, layers: [] },
    center: [0, 0],
    zoom: 0,
  });

  map.on('load', () => {
    map.addSource('composite', {
      type: 'vector',
      tiles: [`${window.location.origin}/tiles/{z}/{x}/{y}.pbf`],
      minzoom: 0,
      maxzoom: 0,
    });
    map.addLayer({
      id: 'buildings',
      type: 'fill',
      source: 'composite',
      'source-layer': 'buildings',
      paint: {
        'fill-color': '#9aa7b8',
        'fill-outline-color': '#4d5766',
      },
    });
    map.addLayer({
      id: 'roads',
      type: 'line',
      source: 'composite',
      'source-layer': 'roads',
      paint: {
        'line-color': '#c2402e',
        'line-width': 2,
      },
    });
  });

  map.on('idle', () => {
    const roads = map.querySourceFeatures('composite', { sourceLayer: 'roads' }).length;
    const buildings = map.querySourceFeatures('composite', { sourceLayer: 'buildings' }).length;
    if (roads > 0 && buildings > 0) {
      setTerminalState('ready', roads, buildings);
    }
  });

  map.on('error', () => {
    setTerminalState('error');
  });
})();
