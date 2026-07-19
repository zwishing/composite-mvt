use std::error::Error;
use std::fs;
use std::path::Path;

use fast_mvt::{
    DEFAULT_EXTENT, MvtCoord, MvtGeometry, MvtLineString, MvtPolygon, MvtResult, MvtTileBuilder,
};

fn roads_tile() -> MvtResult<Vec<u8>> {
    let geometry = MvtGeometry::LineString(MvtLineString::new(vec![
        MvtCoord { x: 512, y: 2048 },
        MvtCoord { x: 3584, y: 2048 },
    ]));
    let mut layer = MvtTileBuilder::new().layer_with_capacity("roads", 1)?;
    layer.extent(DEFAULT_EXTENT);
    let mut feature = layer.feature(&geometry)?;
    feature.id(Some(1));
    feature.tag_string("kind", "arterial")?;
    Ok(feature.end().end().encode())
}

fn buildings_tile() -> MvtResult<Vec<u8>> {
    let exterior = MvtLineString::new(vec![
        MvtCoord { x: 1536, y: 1536 },
        MvtCoord { x: 2560, y: 1536 },
        MvtCoord { x: 2560, y: 2560 },
        MvtCoord { x: 1536, y: 2560 },
    ]);
    let geometry = MvtGeometry::Polygon(MvtPolygon::new(exterior, Vec::new()));
    let mut layer = MvtTileBuilder::new().layer_with_capacity("buildings", 1)?;
    layer.extent(DEFAULT_EXTENT);
    let mut feature = layer.feature(&geometry)?;
    feature.id(Some(2));
    feature.tag_string("kind", "residential")?;
    Ok(feature.end().end().encode())
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("maplibre")
        .join("fixtures");
    fs::create_dir_all(&directory)?;
    fs::write(directory.join("roads.pbf"), roads_tile()?)?;
    fs::write(directory.join("buildings.pbf"), buildings_tile()?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use fast_mvt::{MvtGeometry, MvtReaderRef, MvtValueRef};

    use super::*;

    #[test]
    fn roads_fixture_contains_the_approved_line_feature() {
        let bytes = roads_tile().unwrap();
        let reader = MvtReaderRef::new(&bytes).unwrap();
        let layer = reader.layers().next().unwrap();
        let feature = layer.features().next().unwrap();

        assert_eq!(reader.layer_count(), 1);
        assert_eq!(layer.name(), "roads");
        assert_eq!(layer.extent(), DEFAULT_EXTENT.get());
        assert_eq!(layer.feature_count(), 1);
        assert_eq!(feature.id(), Some(1));
        assert!(matches!(
            feature.geometry().unwrap(),
            MvtGeometry::LineString(_)
        ));
        assert_eq!(
            feature.properties_vec().unwrap(),
            vec![("kind", MvtValueRef::String("arterial"))]
        );
    }

    #[test]
    fn buildings_fixture_contains_the_approved_polygon_feature() {
        let bytes = buildings_tile().unwrap();
        let reader = MvtReaderRef::new(&bytes).unwrap();
        let layer = reader.layers().next().unwrap();
        let feature = layer.features().next().unwrap();

        assert_eq!(reader.layer_count(), 1);
        assert_eq!(layer.name(), "buildings");
        assert_eq!(layer.extent(), DEFAULT_EXTENT.get());
        assert_eq!(layer.feature_count(), 1);
        assert_eq!(feature.id(), Some(2));
        assert!(matches!(
            feature.geometry().unwrap(),
            MvtGeometry::Polygon(_)
        ));
        assert_eq!(
            feature.properties_vec().unwrap(),
            vec![("kind", MvtValueRef::String("residential"))]
        );
    }
}
