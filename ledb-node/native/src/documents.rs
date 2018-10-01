use ledb::{Document, Result, Value};
use neon::prelude::*;
use neon_serde::to_value;
use std::mem::replace;
use std::usize;

pub struct Documents(pub(crate) Option<Box<Iterator<Item = Result<Document<Value>>>>>);

static INVALID_RANGE: &'static str = "Argument not in range 0..N";
static INVALID_ITERATOR: &'static str = "Invalid documents iterator";

declare_types! {
    /// An iterable documents
    pub class JsDocuments for Documents {
        init(_cx) {
            Ok(Documents(None))
        }

        method skip(mut cx) {
            let num = cx.argument::<JsNumber>(0)?.value();

            if num < 0.0 || num > usize::MAX as f64 {
                return cx.throw_range_error(INVALID_RANGE);
            }

            let num: usize = num as usize;

            let mut this = cx.this();

            js_try!(cx, {
                let guard = cx.lock();
                let mut this = this.borrow_mut(&guard);

                if let Some(iter) = replace(&mut this.0, None) {
                    this.0 = Some(Box::new(iter.skip(num)));
                    Ok(())
                } else {
                    Err(INVALID_ITERATOR)
                }
            });

            Ok(this.upcast())
        }

        method take(mut cx) {
            let num = cx.argument::<JsNumber>(0)?.value();

            if num < 0.0 || num > usize::MAX as f64 {
                return cx.throw_range_error(INVALID_RANGE);
            }

            let num: usize = num as usize;

            let mut this = cx.this();

            js_try!(cx, {
                let guard = cx.lock();
                let mut this = this.borrow_mut(&guard);

                if let Some(iter) = replace(&mut this.0, None) {
                    this.0 = Some(Box::new(iter.take(num)));
                    Ok(())
                } else {
                    Err(INVALID_ITERATOR)
                }
            });

            Ok(this.upcast())
        }

        method end(mut cx) {
            let this = cx.this();

            let status = {
                let guard = cx.lock();
                let this = this.borrow(&guard);
                this.0.is_none()
            };

            Ok(cx.boolean(status).upcast())
        }

        method next(mut cx) {
            let mut this = cx.this();

            let doc: Option<Document<Value>> = js_try!(cx, {
                let guard = cx.lock();
                let mut this = this.borrow_mut(&guard);

                let doc = if let Some(iter) = &mut this.0 {
                    iter.next().map_or(Ok(None), |res| res.map(Some))
                } else {
                    Err(INVALID_ITERATOR.into())
                };

                match doc {
                    Ok(None) => {
                        // invalidate iterator
                        this.0 = None;
                        Ok(None)
                    },
                    Ok(Some(doc)) => Ok(Some(doc)),
                    Err(err) => Err(err),
                }
            });

            Ok(js_try!(cx, to_value(&mut cx, &doc)).upcast())
        }

        method collect(mut cx) {
            let mut this = cx.this();

            let docs: Vec<Document<Value>> = js_try!(cx, {
                let guard = cx.lock();
                let mut this = this.borrow_mut(&guard);

                if let Some(iter) = replace(&mut this.0, None) {
                    iter.collect::<Result<Vec<_>>>()
                } else {
                    Err(INVALID_ITERATOR.into())
                }
            });

            Ok(js_try!(cx, to_value(&mut cx, &docs)).upcast())
        }

        method count(mut cx) {
            let this = cx.this();

            let count = js_try!(cx, {
                let guard = cx.lock();
                let this = this.borrow(&guard);
                if let Some(iter) = &this.0 {
                    Ok(iter.size_hint().0)
                } else {
                    Err(INVALID_ITERATOR)
                }
            });

            Ok(cx.number(count as f64).upcast())
        }
    }
}
