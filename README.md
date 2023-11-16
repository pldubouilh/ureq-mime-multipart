# ureq-mime-multipart
ureq mime based multipart toolkit

```rust
use ureq_multipart::MultipartRequest;

let resp  = ureq::post("https://test.url")
    .send_multipart_file("name","1.txt")?
    .into_json()?;
```

[mostly from the multipart crate](https://crates.io/crates/ureq_multipart) with added tests & a few small fixups ([website](https://gitee.com/eshangrao/ureq-multipart-toolkit))