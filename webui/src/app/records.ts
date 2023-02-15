export interface Value {
    Weight?: number,
    FatPercent?: number,
}

export interface Record {
    timestamp: string,
    values: Value[],
}
