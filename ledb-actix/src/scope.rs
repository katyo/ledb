use actix::Addr;
use actix_web::error::{
    Error, ErrorBadRequest, ErrorInternalServerError, ErrorNotFound, ErrorServiceUnavailable,
};
use actix_web::{http::Method, HttpRequest, HttpResponse, Json, Path, Query, Scope, State};
use futures::{
    future::{result, Either},
    Future,
};
use serde_with::json::nested as json_str;
use std::usize;

use super::{
    Delete, Document, DropCollection, DropIndex, EnsureCollection, EnsureIndex, Filter, Find, Get,
    GetCollections, GetIndexes, GetInfo, GetStats, IndexKind, Info, Insert, KeyType,
    ListCollections, Modify, Order, Primary, Put, Remove, Stats, Storage, Update, Value,
};

/// Storage actor address type
pub type StorageAddr = Addr<Storage>;

/// Scoped storage adapter for **actix-web**
pub fn storage(scope: Scope<StorageAddr>) -> Scope<StorageAddr> {
    scope
        .resource("", |res| res.get().with(get_usage))
        .resource("/info", |res| {
            res.name("info");
            res.get().with_async(get_info);
        }).resource("/stats", |res| {
            res.name("stats");
            res.get().with_async(get_stats);
        }).nested("/collection", |scope| {
            scope
                .resource("", |res| {
                    res.name("collections");
                    res.get().with_async(get_collections);
                    res.post().with_async(ensure_collection);
                }).nested("/{collection}", |scope| {
                    scope
                        .resource("", |res| {
                            res.name("collection");
                            res.delete().with_async(drop_collection);
                            // shortcuts for document methods
                            res.post().with_async(insert_document);
                            res.get().with_async(find_documents);
                            res.method(Method::PATCH).with_async(update_documents);
                            res.put().with_async(remove_documents);
                        }).nested("/index", |scope| {
                            scope
                                .resource("", |res| {
                                    res.name("indexes");
                                    res.get().with_async(get_indexes);
                                    res.post().with_async(ensure_index);
                                }).resource("/{index}", |res| {
                                    res.name("index");
                                    res.delete().with_async(drop_index);
                                })
                        }).nested("/document", |scope| {
                            scope
                                .resource("", |res| {
                                    res.name("documents");
                                    res.post().with_async(insert_document);
                                    res.get().with_async(find_documents);
                                    res.put().with_async(update_documents);
                                    res.delete().with_async(remove_documents);
                                }).resource("/{id}", |res| {
                                    res.name("document");
                                    res.get().with_async(get_document);
                                    res.put().with_async(put_document);
                                    res.delete().with_async(delete_document);
                                })
                        }).resource("/{id}", |res| {
                            res.name("document_short");
                            res.get().with_async(get_document);
                            res.put().with_async(put_document);
                            res.delete().with_async(delete_document);
                        })
                })
        })
}

/// Usage info handler
pub fn get_usage(req: HttpRequest<StorageAddr>) -> String {
    format!(
        r#"LEDB HTTP interface {version}

Storage API:

    # get database info
    GET {info}
    # get database statistics
    GET {stats}

Collection API:

    # get list of collections
    GET {collections}
    # create new empty collection
    POST {collections}?name=$collection_name
    # drop collection with all documents
    DELETE {collection}

Index API:

    # get indexes of collection
    GET {indexes}
    # create new index for collection
    POST {indexes}?name=$field_path&kind=$index_kind&type=$key_type
    # drop index of collection
    DELETE {index}

Document API:

    # find documents using query
    GET {documents}?filter=$query&order=$ordering&offset=10&length=10
    GET {collection}?filter=$query&order=$ordering&offset=10&length=10
    # modify documents using query
    PUT {documents}?filter=$query&modify=$modifications
    PATCH {collection}?filter=$query&modify=$modifications
    # remove documents using query
    DELETE {documents}?filter=$query
    PUT {collection}?filter=$query

    # insert new document
    POST {documents}
    POST {collection}
    # get document by id
    GET {document}
    GET {document_short}
    # replace document
    PUT {document}
    PUT {document_short}
    # remove document
    DELETE {document}
    DELETE {document_short}

Supported index kinds:

    index -- Normal index which may contain duplicated keys
    unique -- Index which contains unique keys only

Supported key types:

    int    -- 64-bit signed integer
    float  -- 64-bit floating point number
    bool   -- boolean value
    string -- UTF-8 string
    binary -- binary data

See documentation: {documentation}
"#,
        version = env!("CARGO_PKG_VERSION"),
        documentation = env!("CARGO_PKG_HOMEPAGE"),
        info = req.url_for_static("info").unwrap(),
        stats = req.url_for_static("stats").unwrap(),
        collections = req.url_for_static("collections").unwrap(),
        collection = req.url_for("collection", &["$collection_name"]).unwrap(),
        indexes = req.url_for("indexes", &["$collection_name"]).unwrap(),
        index = req
            .url_for("document", &["$collection_name", "$index_name"])
            .unwrap(),
        documents = req.url_for("documents", &["$collection_name"]).unwrap(),
        document = req
            .url_for("document", &["$collection_name", "$document_id"])
            .unwrap(),
        document_short = req
            .url_for("document_short", &["$collection_name", "$document_id"])
            .unwrap(),
    )
}

/// Storage info handler
pub fn get_info(addr: State<StorageAddr>) -> impl Future<Item = Json<Info>, Error = Error> {
    addr.send(GetInfo)
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

/// Storage stats handler
pub fn get_stats(addr: State<StorageAddr>) -> impl Future<Item = Json<Stats>, Error = Error> {
    addr.send(GetStats)
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

/// Storage collections handler
pub fn get_collections(
    addr: State<StorageAddr>,
) -> impl Future<Item = Json<ListCollections>, Error = Error> {
    addr.send(GetCollections)
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

/// Collection parameters
#[derive(Serialize, Deserialize)]
pub struct CollectionParams {
    pub name: String,
}

/// Ensure collection handler
pub fn ensure_collection(
    (addr, params, req): (
        State<StorageAddr>,
        Query<CollectionParams>,
        HttpRequest<StorageAddr>,
    ),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let CollectionParams { name } = params.into_inner();
    if let Ok(url) = req.url_for("collection", &[&name]) {
        Either::A(
            addr.send(EnsureCollection(name))
                .map_err(ErrorServiceUnavailable)
                .and_then(|res| res.map_err(ErrorInternalServerError))
                .map(move |res| {
                    if res {
                        HttpResponse::Created()
                    } else {
                        HttpResponse::Ok()
                    }.header("location", url.as_str())
                    .finish()
                }),
        )
    } else {
        Either::B(result(Err(ErrorBadRequest("Invalid collection name"))))
    }
}

/// Drop collection handler
pub fn drop_collection(
    (addr, coll): (State<StorageAddr>, Path<String>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    addr.send(DropCollection(coll.into_inner()))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(|res| {
            if res {
                Ok(HttpResponse::NoContent().finish())
            } else {
                Err(ErrorNotFound("Collection not found"))
            }
        })
}

/// Index parameters
#[derive(Serialize, Deserialize)]
pub struct IndexParams {
    pub name: String,
    #[serde(default)]
    pub kind: IndexKind,
    #[serde(rename = "type")]
    pub key: KeyType,
}

/// Get indexes handler
pub fn get_indexes(
    (addr, coll): (State<StorageAddr>, Path<String>),
) -> impl Future<Item = Json<Vec<IndexParams>>, Error = Error> {
    addr.send(GetIndexes(coll.into_inner()))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|indexes| {
            Json(
                indexes
                    .into_iter()
                    .map(|(name, kind, key)| IndexParams {
                        name: String::from(name.as_ref()),
                        kind,
                        key,
                    }).collect(),
            )
        })
}

/// Ensure index handler
pub fn ensure_index(
    (addr, coll, params, req): (
        State<StorageAddr>,
        Path<String>,
        Query<IndexParams>,
        HttpRequest<StorageAddr>,
    ),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let IndexParams { name, kind, key } = params.into_inner();
    if let Ok(url) = req.url_for("index", &[&coll, &name]) {
        Either::A(
            addr.send(EnsureIndex(coll.into_inner(), name, kind, key))
                .map_err(ErrorServiceUnavailable)
                .and_then(|res| res.map_err(ErrorInternalServerError))
                .map(move |res| {
                    if res {
                        HttpResponse::Created()
                    } else {
                        HttpResponse::Ok()
                    }.header("location", url.as_str())
                    .finish()
                }),
        )
    } else {
        Either::B(result(Err(ErrorBadRequest("Invalid index name"))))
    }
}

/// Drop index handler
pub fn drop_index(
    (addr, path): (State<StorageAddr>, Path<(String, String)>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let (coll, idx) = path.into_inner();
    addr.send(DropIndex(coll, idx))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(|res| {
            if res {
                Ok(HttpResponse::NoContent().finish())
            } else {
                Err(ErrorNotFound("Index not found"))
            }
        })
}

/// Insert document handler
pub fn insert_document(
    (addr, coll, doc, req): (
        State<StorageAddr>,
        Path<String>,
        Json<Value>,
        HttpRequest<StorageAddr>,
    ),
) -> impl Future<Item = HttpResponse, Error = Error> {
    addr.send(Insert(&*coll, doc.into_inner()))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(move |id| {
            req.url_for("document", &[&coll.into_inner(), &id.to_string()])
                .map_err(ErrorInternalServerError)
        }).map(|url| {
            HttpResponse::Created()
                .header("location", url.as_str())
                .finish()
        })
}

/// Find query parameters
#[derive(Serialize, Deserialize)]
pub struct FindParams {
    #[serde(default)]
    #[serde(with = "json_str")]
    pub filter: Option<Filter>,
    #[serde(default)]
    #[serde(with = "json_str")]
    pub order: Order,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub length: Option<usize>,
}

/// Find documents query handler
pub fn find_documents(
    (addr, coll, query): (State<StorageAddr>, Path<String>, Query<FindParams>),
) -> impl Future<Item = Json<Vec<Document<Value>>>, Error = Error> {
    let FindParams {
        filter,
        order,
        offset,
        length,
    } = query.into_inner();
    addr.send(Find::<_, Value>(coll.into_inner(), filter, order))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(move |docs| {
            docs.skip(offset.unwrap_or(0))
                .take(length.unwrap_or(usize::MAX))
                .collect::<Result<Vec<_>, _>>()
                .map_err(ErrorInternalServerError)
                .map(Json)
        }).map_err(ErrorInternalServerError)
}

/// Update query parameters
#[derive(Serialize, Deserialize)]
pub struct UpdateParams {
    #[serde(default)]
    #[serde(with = "json_str")]
    pub filter: Option<Filter>,
    pub modify: Modify,
}

/// Update documents query handler
pub fn update_documents(
    (addr, coll, query): (State<StorageAddr>, Path<String>, Query<UpdateParams>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let UpdateParams { filter, modify } = query.into_inner();
    addr.send(Update(coll.into_inner(), filter, modify))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|affected_docs| {
            HttpResponse::NoContent()
                .header("affected", affected_docs.to_string())
                .finish()
        })
}

/// Remove query parameters
#[derive(Serialize, Deserialize)]
pub struct RemoveParams {
    #[serde(default)]
    #[serde(with = "json_str")]
    pub filter: Option<Filter>,
}

/// Remove documents query handler
pub fn remove_documents(
    (addr, coll, query): (State<StorageAddr>, Path<String>, Query<RemoveParams>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let RemoveParams { filter } = query.into_inner();
    addr.send(Remove(coll.into_inner(), filter))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|affected_docs| {
            HttpResponse::NoContent()
                .header("affected", affected_docs.to_string())
                .finish()
        })
}

/// Get document handler
pub fn get_document(
    (addr, path): (State<StorageAddr>, Path<(String, Primary)>),
) -> impl Future<Item = Json<Document<Value>>, Error = Error> {
    let (coll, id) = path.into_inner();
    addr.send(Get(coll, id))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(|res| {
            res.map(Json)
                .ok_or_else(|| ErrorNotFound("Document not found"))
        })
}

/// Put document handler
pub fn put_document(
    (addr, path, data): (State<StorageAddr>, Path<(String, Primary)>, Json<Value>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let (coll, id) = path.into_inner();
    addr.send(Put(coll, Document::new(data.into_inner()).with_id(id)))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|_| HttpResponse::NoContent().finish())
}

/// Delete document handler
pub fn delete_document(
    (addr, path): (State<StorageAddr>, Path<(String, Primary)>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let (coll, id) = path.into_inner();
    addr.send(Delete(coll, id))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(|res| {
            if res {
                Ok(HttpResponse::NoContent().finish())
            } else {
                Err(ErrorNotFound("Document not found"))
            }
        })
}
