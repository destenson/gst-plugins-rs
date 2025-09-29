import { APIClient } from '../client.ts';

export interface TableInfo {
  name: string;
  rowCount: number;
  columns: string[];
}

export interface DatabaseRecord {
  [key: string]: any;
}

export interface UpdateRequest {
  data: DatabaseRecord;
  where_clause: DatabaseRecord;
}

export class DatabaseAPI {
  constructor(private client: APIClient) {}

  /**
   * List all tables in the database
   */
  async listTables(): Promise<TableInfo[]> {
    return this.client.get<TableInfo[]>('/api/v1/database/tables');
  }

  /**
   * Get all data from a specific table
   */
  async getTableData(tableName: string): Promise<DatabaseRecord[]> {
    return this.client.get<DatabaseRecord[]>(`/api/v1/database/tables/${tableName}`);
  }

  /**
   * Update a record in a table
   */
  async updateRecord(tableName: string, request: UpdateRequest): Promise<{ rows_affected: number }> {
    return this.client.post<{ rows_affected: number }>(`/api/v1/database/tables/${tableName}/update`, request);
  }

  /**
   * Insert a new record into a table
   */
  async insertRecord(tableName: string, data: DatabaseRecord): Promise<{ rows_affected: number; last_insert_rowid?: number }> {
    return this.client.post<{ rows_affected: number; last_insert_rowid?: number }>(`/api/v1/database/tables/${tableName}/insert`, data);
  }

  /**
   * Delete a record from a table
   */
  async deleteRecord(tableName: string, whereClause: DatabaseRecord): Promise<{ rows_affected: number }> {
    return this.client.post<{ rows_affected: number }>(`/api/v1/database/tables/${tableName}/delete`, whereClause);
  }

  /**
   * Clear all data from a table
   */
  async clearTable(tableName: string): Promise<{ rows_affected: number }> {
    return this.client.post<{ rows_affected: number }>(`/api/v1/database/tables/${tableName}/clear`, {});
  }
}
