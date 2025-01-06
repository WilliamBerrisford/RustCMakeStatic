#include "cmtest/include/bridge.h"
#include "main.h"
#include <iostream>

BlobstoreClient::BlobstoreClient() {}

std::unique_ptr<BlobstoreClient> new_blobstore_client() {
  std::cout << "Hello Bridge\n";
  hello();
  return std::unique_ptr<BlobstoreClient>(new BlobstoreClient());
}
