extern crate ledb;

#[macro_use]
extern crate neon;

extern crate neon_serde;

#[macro_use]
mod helper;
mod collection;
mod documents;
mod storage;
mod refine;

use collection::JsCollection;
use documents::JsDocuments;
use ledb::Storage;
use neon::prelude::*;
use neon_serde::to_value;
use storage::JsStorage;
use refine::refine;

fn list_openned_storages(mut cx: FunctionContext) -> JsResult<JsValue> {
    let list = js_try!(cx, Storage::openned());
    Ok(js_try!(cx, to_value(&mut cx, &list)))
}

register_module!(mut cx, {
    cx.export_function("openned", list_openned_storages)?;
    cx.export_class::<JsStorage>("Storage")?;
    cx.export_class::<JsCollection>("Collection")?;
    cx.export_class::<JsDocuments>("Documents")?;
    Ok(())
});
