export type Primary = number;

export interface GenericDocument {}

export type Document<T extends GenericDocument> = {
    $: Primary;
} & T;

export interface Documents<T> {
    skip(num: number): Documents<T>;
    take(num: number): Documents<T>;
    next(): Document<T> | void;
    end(): boolean;
    collect(): Document<T>[];
    count(): number;
}

export type KeyType
    = 'int'
    | 'float'
    | 'bool'
    | 'string'
    | 'binary'
    ;

export type KeyData = number | string | boolean | ArrayBufferLike;

export type Value = any;

export type IndexKind = 'uni' | 'dup';

export type Index = [
    string, // field
    IndexKind, // kind
    KeyType // type
];

export type Filter
    = FilterCond
    | { [field: string]: FilterComp }
    | FilterNone
    ;

export type FilterCond
    = FilterAnd
    | FilterOr
    | FilterNot
    ;

export interface FilterAnd { $and: Filter[] }
export interface FilterOr  { $or:  Filter[] }
export interface FilterNot { $not: Filter }

export type FilterComp
    = FilterEq
    | FilterIn
    | FilterLt
    | FilterLe
    | FilterGt
    | FilterGe
    | FilterBw
    | FilterHas
    ;

export interface FilterEq { $eq: KeyData }
export interface FilterIn { $in: KeyData[] }
export interface FilterLt { $lt: KeyData }
export interface FilterLe { $le: KeyData }
export interface FilterGt { $gt: KeyData }
export interface FilterGe { $ge: KeyData }
export interface FilterBw { $in: [KeyData, boolean, KeyData, boolean] }

export type FilterHas = '$has';

export type FilterNone = null;

export type Order
    = OrderByPrimary
    | OrderByField;

export type OrderByPrimary = OrderKind;
export type OrderByField = [string, OrderKind];

export type OrderKind = '$asc' | '$desc';

export type Modify = [string, Action][];

export type Action
    = ActionSet
    | ActionDelete
    | ActionAdd
    | ActionSub
    | ActionMul
    | ActionDiv
    | ActionToggle
    | ActionReplace
    | ActionMerge
    ;

export interface ActionSet { $set: Value }
export type ActionDelete = '$delete';

export interface ActionAdd { $add: Value }
export interface ActionSub { $sub: Value }
export interface ActionMul { $mul: Value }
export interface ActionDiv { $div: Value }

export type ActionToggle = '$toggle';

export interface ActionReplace { $replace: [string, string] }
export interface ActionSplice { $splice: [number, number, ...Value[]] }
export interface ActionMerge { $merge: Value }

// Storage info
export interface Info {
    map_size: number,
    last_page: number,
    last_transaction: number,
    max_readers: number,
    num_readers: number,
}

// Storage stats
export interface Stats {
    page_size: number,
    btree_depth: number,
    branch_pages: number,
    leaf_pages: number,
    overflow_pages: number,
    data_entries: number,
}

// Storage options
export interface Options {
    // options
    map_size?: number,
    max_readers?: number,
    max_dbs?: number,
    // flags
    map_async?: boolean,
    no_lock?: boolean,
    no_mem_init?: boolean,
    no_meta_sync?: boolean,
    no_read_ahead?: boolean,
    no_sub_dir?: boolean,
    no_sync?: boolean,
    no_tls?: boolean,
    read_only?: boolean,
    write_map?: boolean,
}

// Storage handle interface
export class Storage {
    constructor(path: string, opts?: Options);
    
    get_info(): Info;
    get_stats(): Stats;

    has_collection(name: string): boolean;
    collection(name: string): Collection;
    drop_collection(name: string): boolean;
    get_collections(): string[];
}

// Collection handle interface
export class Collection {
    constructor(storage: Storage, name: string);
    
    insert<T extends GenericDocument>(doc: T): Primary;
    find<T extends GenericDocument>(filter: Filter, order?: Order): Documents<T>;
    update(filter: Filter, modify: Modify): number;
    remove(filter: Filter): number;

    dump<T extends GenericDocument>(): Documents<T>;
    load<T extends GenericDocument>(docs: Documents<T>): number;

    purge(): void;

    has(id: Primary): boolean;
    get<T extends GenericDocument>(id: Primary): T | null;
    put<T extends GenericDocument>(doc: T): void;
    delete(id: Primary): boolean;

    get_indexes(): Index[];
    set_indexes(indexes: Index[]): void;
    has_index(field: string): void;
    ensure_index(field: string, kind: IndexKind, type: KeyType): boolean;
    drop_index(field: string): boolean;
}

// Get openned databases
export function openned(): string[];
