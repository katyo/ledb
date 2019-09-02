use neon::prelude::*;
use neon_serde::{from_value, to_value};

use ledb::{Options, Storage};

use super::JsCollection;

declare_types! {
    /// A storage class
    pub class JsStorage for Storage {
        init(mut cx) {
            let path = cx.argument::<JsString>(0)?.value();
            let opts = if let Some(opts) = cx.argument_opt(1) {
                from_value(&mut cx, opts)?
            } else {
                Options::default()
            };
            Ok(js_try!(cx, Storage::new(&path, opts)))
        }

        method get_stats(mut cx) {
            let this = cx.this();
            let stats = js_try!(cx, {
                let guard = cx.lock();
                let storage = this.borrow(&guard);
                storage.get_stats()
            });
            Ok(to_value(&mut cx, &stats)?)
        }

        method get_info(mut cx) {
            let this = cx.this();
            let stats = js_try!(cx, {
                let guard = cx.lock();
                let storage = this.borrow(&guard);
                storage.get_info()
            });
            Ok(to_value(&mut cx, &stats)?)
        }

        method has_collection(mut cx) {
            let name = cx.argument::<JsString>(0)?.value();
            let this = cx.this();
            let has = js_try!(cx, {
                let guard = cx.lock();
                let storage = this.borrow(&guard);
                storage.has_collection(&name)
            });
            Ok(cx.boolean(has).upcast())
        }

        method collection(mut cx) {
            let name = cx.argument::<JsString>(0)?;
            let this = cx.this();
            Ok(JsCollection::new(&mut cx, vec![this.upcast::<JsValue>(), name.upcast::<JsValue>()])?.upcast())
        }

        method drop_collection(mut cx) {
            let name = cx.argument::<JsString>(0)?.value();
            let this = cx.this();
            let has = js_try!(cx, {
                let guard = cx.lock();
                let storage = this.borrow(&guard);
                storage.drop_collection(&name)
            });
            Ok(cx.boolean(has).upcast())
        }

        method get_collections(mut cx) {
            let this = cx.this();
            let list = js_try!(cx, {
                let guard = cx.lock();
                let storage = this.borrow(&guard);
                storage.get_collections()
            });
            Ok(js_try!(cx, to_value(&mut cx, &list)))
        }
    }
}
