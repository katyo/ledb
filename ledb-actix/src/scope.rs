use actix::Addr;
use actix_web::{
    error::{
        Error, ErrorBadRequest, ErrorInternalServerError, ErrorNotFound, ErrorServiceUnavailable,
    },
    HttpRequest, HttpResponse,
    Scope,
    web::{Json, Path, Query, Data, scope, resource, get, post, put, delete, patch},
};
use futures::{
    future::{result, Either},
    Future,
};
use serde_with::json::nested as json_str;
use std::usize;
use serde::{Serialize, Deserialize};

use super::{
    Delete, Document, DropCollection, DropIndex, EnsureCollection, EnsureIndex, Filter, Find, Get,
    GetCollections, GetIndexes, GetInfo, GetStats, Info, Insert, KeyField, ListCollections, Modify,
    Order, Primary, Put, Remove, Stats, Storage, Update, Value,
};

/// Storage actor address type
pub type StorageAddr = Addr<Storage>;

/// Scoped storage adapter for **actix-web**
pub fn storage() -> Scope {
    scope("")
        .service(
            resource("/")
                .name("usage")
                .route(get().to(get_usage))
        )
        .service(
            resource("/info")
                .name("info")
                .route(get().to_async(get_info))
        )
        .service(
            resource("/stats")
                .name("stats")
                .route(get().to_async(get_stats))
        )
        .service(
            resource("/collection")
                .name("collections")
                .route(get().to_async(get_collections))
                .route(post().to_async(ensure_collection))
        )
        .service(
            scope("/collection")
                .service(
                    resource("/{collection}")
                        .name("collection")
                        .route(delete().to_async(drop_collection))
                        // shortcuts for document methods
                        .route(post().to_async(insert_document))
                        .route(get().to_async(find_documents))
                        .route(patch().to_async(update_documents))
                        .route(put().to_async(remove_documents))
                )
                .service(
                    scope("/{collection}")
                        .service(
                            resource("/index")
                                .name("indexes")
                                .route(get().to_async(get_indexes))
                                .route(post().to_async(ensure_index))
                        )
                        .service(
                            scope("/index")
                                .service(
                                    resource("/{index}")
                                        .name("index")
                                        .route(delete().to_async(drop_index))
                                )
                        )
                        .service(
                            resource("/document")
                                .name("documents")
                                .route(post().to_async(insert_document))
                                .route(get().to_async(find_documents))
                                .route(put().to_async(update_documents))
                                .route(delete().to_async(remove_documents))
                        )
                        .service(
                            scope("/document")
                                .service(
                                    resource("/{id}")
                                        .name("document")
                                        .route(get().to_async(get_document))
                                        .route(put().to_async(put_document))
                                        .route(delete().to_async(delete_document))
                                )
                        )
                        .service(
                            resource("/{id}")
                                .name("document_short")
                                .route(get().to_async(get_document))
                                .route(put().to_async(put_document))
                                .route(delete().to_async(delete_document))
                        )
                )
        )
}

/// Usage info handler
pub fn get_usage(req: HttpRequest) -> String {
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
    POST {indexes}?path=$field_path&kind=$index_kind&key=$key_type
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
pub fn get_info(addr: Data<StorageAddr>) -> impl Future<Item = Json<Info>, Error = Error> {
    addr.send(GetInfo)
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

/// Storage stats handler
pub fn get_stats(addr: Data<StorageAddr>) -> impl Future<Item = Json<Stats>, Error = Error> {
    addr.send(GetStats)
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

/// Storage collections handler
pub fn get_collections(
    addr: Data<StorageAddr>,
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
    addr: Data<StorageAddr>,
    params: Query<CollectionParams>,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let CollectionParams { name } = params.into_inner();
    match req.url_for("collection", &[&name]) {
        Ok(url) => Either::A(
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
                })
        ),
        Err(error) => Either::B(
            result(Err(ErrorBadRequest(format!("Cannot get url for collection ({})", error) /*"Invalid collection name"*/)))
        )
    }
}

/// Drop collection handler
pub fn drop_collection(
    addr: Data<StorageAddr>,
    coll: Path<String>,
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

/// Get indexes handler
pub fn get_indexes(
    addr: Data<StorageAddr>,
    coll: Path<String>,
) -> impl Future<Item = Json<Vec<KeyField>>, Error = Error> {
    addr.send(GetIndexes(coll.into_inner()))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|indexes| Json(indexes.into_iter().collect()))
}

/// Ensure index handler
pub fn ensure_index(
    addr: Data<StorageAddr>,
    coll: Path<String>,
    params: Query<KeyField>,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let KeyField { path, kind, key } = params.into_inner();
    if let Ok(url) = req.url_for("index", &[&coll, &path]) {
        Either::A(
            addr.send(EnsureIndex(coll.into_inner(), path, kind, key))
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
    addr: Data<StorageAddr>,
    path: Path<(String, String)>,
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
    addr: Data<StorageAddr>,
    coll: Path<String>,
    doc: Json<Value>,
    req: HttpRequest,
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
    addr: Data<StorageAddr>,
    coll: Path<String>,
    query: Query<FindParams>,
) -> impl Future<Item = Json<Vec<Value>>, Error = Error> {
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
    addr: Data<StorageAddr>,
    coll: Path<String>,
    query: Query<UpdateParams>,
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
    (addr, coll, query): (Data<StorageAddr>, Path<String>, Query<RemoveParams>),
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
    addr: Data<StorageAddr>,
    path: Path<(String, Primary)>,
) -> impl Future<Item = Json<Value>, Error = Error> {
    let (coll, id) = path.into_inner();
    addr.send(Get(coll, id))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(|res| {
            res.map(Json)
                .ok_or_else(|| ErrorNotFound("Document not found"))
        })
}

#[derive(Serialize)]
pub struct DocumentWithId {
    #[serde(rename = "$")]
    id: Primary,
    #[serde(flatten)]
    val: Value,
}

impl Document for DocumentWithId {}

/// Put document handler
pub fn put_document(
    addr: Data<StorageAddr>,
    path: Path<(String, Primary)>,
    data: Json<Value>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let (coll, id) = path.into_inner();
    let doc = DocumentWithId {
        id,
        val: data.into_inner(),
    };
    addr.send(Put(coll, doc))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|_| HttpResponse::NoContent().finish())
}

/// Delete document handler
pub fn delete_document(
    addr: Data<StorageAddr>,
    path: Path<(String, Primary)>,
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
