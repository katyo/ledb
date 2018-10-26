use ledb::{Collection, Filter, Identifier, IndexKind, KeyType, Modify, Order, Primary, Value};
use neon::prelude::*;
use neon_serde::{from_value, to_value};
use std::u32;

use super::{JsDocuments, JsStorage};

declare_types! {
    /// A collection class
    pub class JsCollection for Collection {
        init(mut cx) {
            let storage = cx.argument::<JsStorage>(0)?;
            let name = cx.argument::<JsString>(1)?.value();
            let collection = js_try!(cx, {
                let guard = cx.lock();
                let storage = storage.borrow(&guard);
                storage.collection(&name)
            });
            Ok(collection)
        }

        method insert(mut cx) {
            let raw = cx.argument(0)?;
            let doc: Value = from_value(&mut cx, raw)?;

            let this = cx.this();

            let id = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.insert(&doc)
            });

            Ok(cx.number(id).upcast())
        }

        method find(mut cx) {
            let filter: Option<Filter> = if let Some(filter) = cx.argument_opt(0) {
                from_value(&mut cx, filter)?
            } else {
                None
            };

            let order: Order = if let Some(order) = cx.argument_opt(1) {
                from_value(&mut cx, order)?
            } else {
                Order::default()
            };

            let this = cx.this();

            let iter = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.find(filter, order)
            });

            let mut docs = JsDocuments::new(&mut cx, vec![JsUndefined::new()])?;

            {
                let guard = cx.lock();
                let mut docs = docs.borrow_mut(&guard);
                docs.0 = Some(Box::new(iter));
            }

            Ok(docs.upcast())
        }

        method update(mut cx) {
            let filter: Option<Filter> = if let Some(filter) = cx.argument_opt(0) {
                from_value(&mut cx, filter)?
            } else {
                None
            };

            let modify_raw = cx.argument(1)?;
            let modify: Modify = from_value(&mut cx, modify_raw)?;

            let this = cx.this();

            let affected = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.update(filter, modify)
            });

            Ok(cx.number(affected as u32).upcast())
        }

        method remove(mut cx) {
            let filter: Option<Filter> = if let Some(filter) = cx.argument_opt(0) {
                from_value(&mut cx, filter)?
            } else {
                None
            };

            let this = cx.this();

            let affected = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.remove(filter)
            });

            Ok(cx.number(affected as u32).upcast())
        }

        method dump(mut cx) {
            let this = cx.this();

            let iter = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.dump()
            });

            let mut docs = JsDocuments::new(&mut cx, vec![JsUndefined::new()])?;

            {
                let guard = cx.lock();
                let mut docs = docs.borrow_mut(&guard);
                docs.0 = Some(Box::new(iter));
            }

            Ok(docs.upcast())
        }

        //method load(mut cx) {}

        method purge(mut cx) {
            let this = cx.this();

            js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.purge()
            });

            Ok(cx.undefined().upcast())
        }

        method has(mut cx) {
            let id = cx.argument::<JsNumber>(0)?.value();

            if id < 1.0 || id > u32::MAX as f64 {
                return cx.throw_range_error("Document id must be in range 1..N");
            }

            let id: Primary = id as Primary;

            let this = cx.this();

            let status = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.has(id)
            });

            Ok(cx.boolean(status).upcast())
        }

        method get(mut cx) {
            let id = cx.argument::<JsNumber>(0)?.value();

            if id < 1.0 || id > u32::MAX as f64 {
                return cx.throw_range_error("Document id must be in range 1..N");
            }

            let id: Primary = id as Primary;

            let this = cx.this();

            let doc: Option<Value> = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.get(id)
            });

            Ok(js_try!(cx, to_value(&mut cx, &doc)).upcast())
        }

        method put(mut cx) {
            let raw = cx.argument(0)?;
            let doc: Value = from_value(&mut cx, raw)?;

            let this = cx.this();

            js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.put(&doc)
            });

            Ok(cx.undefined().upcast())
        }

        method delete(mut cx) {
            let id = cx.argument::<JsNumber>(0)?.value();

            if id < 1.0 || id > u32::MAX as f64 {
                return cx.throw_range_error("Document id must be in range 1..N");
            }

            let id: Primary = id as Primary;

            let this = cx.this();

            let status = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.delete(id)
            });

            Ok(cx.boolean(status).upcast())
        }

        method get_indexes(mut cx) {
            let this = cx.this();

            let indexes = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.get_indexes()
            });

            Ok(js_try!(cx, to_value(&mut cx, &indexes)).upcast())
        }

        method set_indexes(mut cx) {
            let indexes = cx.argument(0)?;
            let indexes: Vec<(String, IndexKind, KeyType)> = from_value(&mut cx, indexes)?;
            let indexes: Vec<(Identifier, IndexKind, KeyType)> = indexes.into_iter().map(|(name, kind, key)| (name.into(), kind, key)).collect();

            let this = cx.this();

            js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.set_indexes(&indexes)
            });

            Ok(cx.undefined().upcast())
        }

        method has_index(mut cx) {
            let path = cx.argument::<JsString>(0)?.value();
            let this = cx.this();
            let has = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.has_index(&path)
            });
            Ok(cx.boolean(has).upcast())
        }

        method ensure_index(mut cx) {
            let path = cx.argument::<JsString>(0)?.value();
            let kind = cx.argument(1)?;
            let kind = from_value(&mut cx, kind)?;
            let key = cx.argument(2)?;
            let key = from_value(&mut cx, key)?;

            let this = cx.this();

            let status = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.ensure_index(path, kind, key)
            });

            Ok(cx.boolean(status).upcast())
        }

        method drop_index(mut cx) {
            let path = cx.argument::<JsString>(0)?.value();
            let this = cx.this();
            let status = js_try!(cx, {
                let guard = cx.lock();
                let collection = this.borrow(&guard);
                collection.drop_index(&path)
            });
            Ok(cx.boolean(status).upcast())
        }
    }
}
