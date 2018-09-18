use std::usize;
use futures::Future;
use actix::{Addr};
use actix_web::{Scope, State, Path, Query, Json, HttpRequest, HttpResponse, http::Method};
use actix_web::error::{Error, ErrorInternalServerError, ErrorServiceUnavailable, ErrorNotFound};

use super::{Storage, GetCollections, ListCollections, DropCollection, GetIndexes, ListIndexes, EnsureIndex, DropIndex, Insert, Get, Put, Delete, Update, Remove, Find, IndexKind, KeyType, Filter, Order, Modify, Primary, Document, Value};

pub type StorageAddr = Addr<Storage>;

pub fn storage(scope: Scope<StorageAddr>) -> Scope<StorageAddr> {
    scope
        .resource("", |res| {
            res.get().with_async(get_collections);
        })
        .nested("/{collection}", |scope| {
            scope
                .resource("", |res| {
                    res.name("collection");
                    res.post().with_async(insert_document);
                    res.get().with_async(find_documents);
                    res.method(Method::PATCH).with_async(update_documents);
                    res.put().with_async(remove_documents);
                    res.delete().with_async(drop_collection);
                })
                .nested("/index", |scope| {
                    scope
                        .resource("", |res| {
                            res.get().with_async(get_indexes);
                        })
                        .resource("/{index}", |res| {
                            res.name("index");
                            res.put().with_async(ensure_index);
                            res.delete().with_async(drop_index);
                        })
                })
                .resource("/{primary}", |res| {
                    res.name("document");
                    res.get().with_async(get_document);
                    res.put().with_async(put_document);
                    res.delete().with_async(delete_document);
                })
        })
}

pub fn get_collections(addr: State<StorageAddr>) -> impl Future<Item = Json<ListCollections>, Error = Error> {
    addr.send(GetCollections)
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

pub fn drop_collection((addr, coll): (State<StorageAddr>, Path<&'static str>)) -> impl Future<Item = Json<bool>, Error = Error> {
    addr.send(DropCollection(*coll))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

pub fn get_indexes((addr, coll): (State<StorageAddr>, Path<&'static str>)) -> impl Future<Item = Json<ListIndexes>, Error = Error> {
    addr.send(GetIndexes(*coll))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

#[derive(Serialize, Deserialize)]
pub struct IndexOpts {
    #[serde(default)]
    kind: IndexKind,
    #[serde(rename = "type")]
    key: KeyType,
}

pub fn ensure_index((addr, coll, idx, opts): (State<StorageAddr>, Path<&'static str>, Path<&'static str>, Json<IndexOpts>)) -> impl Future<Item = Json<bool>, Error = Error> {
    addr.send(EnsureIndex(*coll, *idx, opts.kind, opts.key))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

pub fn drop_index((addr, coll, idx): (State<StorageAddr>, Path<&'static str>, Path<&'static str>)) -> impl Future<Item = Json<bool>, Error = Error> {
    addr.send(DropIndex(*coll, *idx))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map(Json).map_err(ErrorInternalServerError))
}

pub fn insert_document((addr, coll, doc, req): (State<StorageAddr>, Path<&'static str>, Json<Value>, HttpRequest<StorageAddr>)) -> impl Future<Item = HttpResponse, Error = Error> {
    addr.send(Insert(*coll, doc.into_inner()))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(move |id| req.url_for("document", &[*coll, &id.to_string()])
                  .map_err(ErrorInternalServerError))
        .map(|url| HttpResponse::Created()
             .header("location", url.as_str())
             .finish())
}

#[derive(Serialize, Deserialize)]
pub struct FindParams {
    #[serde(default)]
    filter: Option<Filter>,
    #[serde(default)]
    order: Order,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    length: Option<usize>,
}

pub fn find_documents((addr, coll, query): (State<StorageAddr>, Path<&'static str>, Query<FindParams>)) -> impl Future<Item = Json<Vec<Document<Value>>>, Error = Error> {
    let FindParams { filter, order, offset, length } = query.into_inner();
    addr.send(Find::<_, Value>(*coll, filter, order))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(move |docs| docs
                  .skip(offset.unwrap_or(0))
                  .take(length.unwrap_or(usize::MAX))
                  .collect::<Result<Vec<_>, _>>()
                  .map_err(ErrorInternalServerError)
                  .map(Json)
        ).map_err(ErrorInternalServerError)
}

#[derive(Serialize, Deserialize)]
pub struct UpdateParams {
    #[serde(default)]
    filter: Option<Filter>,
    modify: Modify,
}

pub fn update_documents((addr, coll, query): (State<StorageAddr>, Path<&'static str>, Query<UpdateParams>)) -> impl Future<Item = HttpResponse, Error = Error> {
    let UpdateParams { filter, modify } = query.into_inner();
    addr.send(Update(*coll, filter, modify))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|affected_docs| HttpResponse::NoContent()
             .header("affected", affected_docs.to_string())
             .finish())
}

#[derive(Serialize, Deserialize)]
pub struct RemoveParams {
    #[serde(default)]
    filter: Option<Filter>,
}

pub fn remove_documents((addr, coll, query): (State<StorageAddr>, Path<&'static str>, Query<RemoveParams>)) -> impl Future<Item = HttpResponse, Error = Error> {
    let RemoveParams { filter } = query.into_inner();
    addr.send(Remove(*coll, filter))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|affected_docs| HttpResponse::NoContent()
             .header("affected", affected_docs.to_string())
             .finish())
}

pub fn get_document((addr, coll, id): (State<StorageAddr>, Path<&'static str>, Path<Primary>)) -> impl Future<Item = Json<Document<Value>>, Error = Error> {
    addr.send(Get(*coll, *id))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(|res| res.map(Json)
                  .ok_or_else(|| ErrorNotFound("Document not found")))
}

pub fn put_document((addr, coll, id, data): (State<StorageAddr>, Path<&'static str>, Path<Primary>, Json<Value>)) -> impl Future<Item = HttpResponse, Error = Error> {
    addr.send(Put(*coll, Document::new(data.into_inner()).with_id(*id)))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .map(|_| HttpResponse::NoContent().finish())
}

pub fn delete_document((addr, coll, id): (State<StorageAddr>, Path<&'static str>, Path<Primary>)) -> impl Future<Item = HttpResponse, Error = Error> {
    addr.send(Delete(*coll, *id))
        .map_err(ErrorServiceUnavailable)
        .and_then(|res| res.map_err(ErrorInternalServerError))
        .and_then(|res| if res {
            Ok(HttpResponse::NoContent().finish())
        } else {
            Err(ErrorNotFound("Document not found"))
        })
}
