use std::sync::Arc;
use std::thread;

#[cfg(feature = "gzip")]
use composite_mvt::Compression;
use composite_mvt::{MvtComposer, MvtSource};

#[cfg(feature = "gzip")]
use super::fixtures;

#[test]
fn arc_composer_is_safe_for_concurrent_requests() {
    // Given: an immutable composer shared by independent request threads.
    let composer = Arc::new(
        MvtComposer::builder()
            .add_source(MvtSource::new("roads").with_layers(["roads"]))
            .build()
            .unwrap(),
    );

    // When: each request composes a distinct payload concurrently.
    let handles: Vec<_> = (0_u8..16)
        .map(|value| {
            let composer = Arc::clone(&composer);
            thread::spawn(move || composer.compose(&[vec![value]]).unwrap())
        })
        .collect();

    // Then: no request observes another request's output.
    for (value, handle) in (0_u8..16).zip(handles) {
        assert_eq!(handle.join().unwrap().as_ref(), &[value]);
    }
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn composer_is_send_and_sync() {
    assert_send_sync::<MvtComposer>();
}

#[cfg(feature = "gzip")]
#[test]
fn compressed_output_is_independent_across_threads() {
    // Given: one immutable gzip composer shared by several request threads.
    let composer = Arc::new(
        MvtComposer::builder()
            .output_compression(Compression::Gzip)
            .add_source(MvtSource::new("roads").with_layers(["roads"]))
            .build()
            .unwrap(),
    );

    // When: every thread compresses its own single-byte payload.
    let handles: Vec<_> = (0_u8..8)
        .map(|value| {
            let composer = Arc::clone(&composer);
            thread::spawn(move || (value, composer.compose(&[vec![value]]).unwrap()))
        })
        .collect();

    // Then: each independently encoded stream decodes to its original payload.
    for handle in handles {
        let (value, output) = handle.join().unwrap();
        assert_eq!(fixtures::gunzip(&output), [value]);
    }
}
