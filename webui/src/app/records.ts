export interface Values {
    weight?: number,
    fatPercent?: number,
    glucose?: number,
    meal?: string,
}

export interface Record {
    timestamp: string,
    values: Values,
}
