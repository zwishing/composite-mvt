# MapLibre MVT fixtures

These two z0 fixtures keep the browser example self-contained. Coordinates are
integer tile-space coordinates in an extent of 4096.

| File | Source layer | Geometry | Coordinates | Feature ID | Property | SHA-256 |
| --- | --- | --- | --- | ---: | --- | --- |
| `roads.pbf` | `roads` | `LineString` | `(512,2048) -> (3584,2048)` | 1 | `kind=arterial` | `a8c803d08eff44ceabb9d58005ea9ffa479fe06e80fde91c2df06443ba4582c1` |
| `buildings.pbf` | `buildings` | `Polygon` | `(1536,1536) -> (2560,1536) -> (2560,2560) -> (1536,2560)` | 2 | `kind=residential` | `4134a34677bb0cedd6233345656e8588413931cd4b9271db4d23c8b1afe1e8ce` |

Their layer names and readability are checked by the `maplibre_server` example tests.

## Downloaded example tiles

`demo/1/{x}/{y}.pbf` and `open/1/{x}/{y}.pbf` are four z=1 tile pairs retained for local
performance tests. The bundled frontend includes 3×3 Europe tile sets at z=2, z=3, z=4, and z=5,
so it works without outbound network access while zooming across those four levels. The demo tiles
retain `geolines`, `centroids`, and `countries`; the OpenFreeMap z=4 tiles include `landuse`.
