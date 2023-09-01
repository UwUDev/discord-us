# Discord Unlimited Storage

## What is this?
This is a simple rust app that allows you to upload files to discord without any file size limit. I know there is some bandwidth limitations per files, but it still really acceptable, and I'll add async upload/download in the future to have parallel uploads/downloads and go faster. Everything is stored in a sqlite database, so you don't have to worry about losing your files. It uses Aes256Cbc encryption to encrypt your files, so you don't have to worry about your files being analysed by discord and tencent + it avoids discord to remove non ToS compliant files.

## How to use it?
Currently, it's only just like a lib so here is simple examples:

### Upload

```rust
use crate::uploader::safe_upload;
use crate::utils::{create_trash_dir, empty_trash};
use crate::database::export_waterfall;

fn main() {
    create_trash_dir(); // Create a trash dir to store the file that will be encrypted and split

    let token = String::from("UHQ discord token");
    let channel_id = 1146787754915676260u64;
    let subscription = Subscription::Free; // Free, Basic, Classic, Boost

    let saved_id = safe_upload("super strong pass", "file path", token, channel_id, subscription);

    // optional if you want to share it with someone
    export_waterfall(saved_id, "cool.waterfall");

    empty_trash(); // Empty the trash dir
}
```

### Download
```rust
use crate::downloader::safe_download;
use crate::utils::{create_trash_dir, empty_trash};

fn main() {
    create_trash_dir(); // Create a trash dir to store the blocks that will be decrypted and merged

    safe_download(2, "super strong password", "out dir"); // 2 is the id of the file in the db
    empty_trash(); // Empty the trash dir
}
```

### Export
```rust
use crate::database::export_waterfall;

fn main() {
    export_waterfall(2, "cool.waterfall"); // 2 is the id of the file in the db
}
```

### Import
```rust
use crate::database::import_waterfall;

fn main() {
    import_waterfall("cool.waterfall");
}
```

### Make sure if you have no db to call `create_db("default pass")` before doing anything else