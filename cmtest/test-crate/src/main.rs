#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("test-crate/include/bridge.h");

        type BlobstoreClient;

        fn new_blobstore_client() -> UniquePtr<BlobstoreClient>;
    }
}

fn main() {
    let _client = ffi::new_blobstore_client();
}
