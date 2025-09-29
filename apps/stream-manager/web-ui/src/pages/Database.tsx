import React, { useState, useEffect } from 'react';
import { useAPI } from '../contexts/APIContext.tsx';
import { Database as DatabaseIcon, Trash2, RefreshCw, Edit, Save, X, Plus, Search } from 'lucide-react';
import LoadingSpinner from '../components/LoadingSpinner.tsx';

import type { TableInfo, DatabaseRecord } from '../api/services/DatabaseAPI.ts';

const Database: React.FC = () => {
  const { api } = useAPI();
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [selectedTable, setSelectedTable] = useState<string | null>(null);
  const [tableData, setTableData] = useState<DatabaseRecord[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [editingRow, setEditingRow] = useState<number | null>(null);
  const [editedData, setEditedData] = useState<DatabaseRecord>({});
  const [searchQuery, setSearchQuery] = useState('');
  const [showAddRow, setShowAddRow] = useState(false);
  const [newRowData, setNewRowData] = useState<DatabaseRecord>({});
  const [error, setError] = useState<string | null>(null);

  // Fetch available tables
  useEffect(() => {
    fetchTables();
  }, []);

  const fetchTables = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const fetchedTables = await api.database.listTables();
      setTables(fetchedTables);
    } catch (err) {
      setError('Failed to fetch tables');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const fetchTableData = async (tableName: string) => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.database.getTableData(tableName);
      setTableData(data);
      setSelectedTable(tableName);

      // Initialize new row data with empty values for all columns
      const tableInfo = tables.find(t => t.name === tableName);
      if (tableInfo) {
        const emptyRow: DatabaseRecord = {};
        tableInfo.columns.forEach(col => {
          emptyRow[col] = '';
        });
        setNewRowData(emptyRow);
      }
    } catch (err) {
      setError(`Failed to fetch data for table ${tableName}`);
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleEdit = (index: number) => {
    setEditingRow(index);
    setEditedData({ ...tableData[index] });
  };

  const handleSave = async (index: number) => {
    try {
      const originalData = tableData[index];
      const whereClause: DatabaseRecord = {};

      // Use the first column as the key (usually 'id')
      const keyColumns = Object.keys(originalData).slice(0, 1);
      keyColumns.forEach(col => {
        whereClause[col] = originalData[col];
      });

      await api.database.updateRecord(selectedTable!, {
        data: editedData,
        where_clause: whereClause
      });

      // Refresh table data
      await fetchTableData(selectedTable!);
      setEditingRow(null);
      setEditedData({});
    } catch (err) {
      setError('Failed to save changes');
      console.error(err);
    }
  };

  const handleCancel = () => {
    setEditingRow(null);
    setEditedData({});
  };

  const handleDelete = async (index: number) => {
    if (!confirm('Are you sure you want to delete this record?')) return;

    try {
      const record = tableData[index];
      const whereClause: DatabaseRecord = {};

      // Use the first column as the key (usually 'id')
      const keyColumns = Object.keys(record).slice(0, 1);
      keyColumns.forEach(col => {
        whereClause[col] = record[col];
      });

      await api.database.deleteRecord(selectedTable!, whereClause);

      // Refresh table data
      await fetchTableData(selectedTable!);
    } catch (err) {
      setError('Failed to delete record');
      console.error(err);
    }
  };

  const handleAddRow = async () => {
    try {
      await api.database.insertRecord(selectedTable!, newRowData);

      // Refresh table data
      await fetchTableData(selectedTable!);
      setShowAddRow(false);
      setNewRowData({});
    } catch (err) {
      setError('Failed to add record');
      console.error(err);
    }
  };

  const handleClearTable = async () => {
    if (!confirm(`Are you sure you want to clear all data from ${selectedTable}? This cannot be undone.`)) return;

    try {
      await api.database.clearTable(selectedTable!);

      // Refresh table data
      await fetchTableData(selectedTable!);
    } catch (err) {
      setError('Failed to clear table');
      console.error(err);
    }
  };

  const filteredData = tableData.filter(row =>
    Object.values(row).some(value =>
      String(value).toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  const getColumnType = (value: any): string => {
    if (value === null || value === undefined) return 'null';
    if (typeof value === 'boolean') return 'boolean';
    if (typeof value === 'number') return 'number';
    if (typeof value === 'string') {
      if (value.match(/^\d{4}-\d{2}-\d{2}T/)) return 'datetime';
      if (value.match(/^(https?|rtsp|ws):\/\//)) return 'url';
    }
    return 'string';
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center space-x-3">
            <DatabaseIcon className="h-8 w-8 text-blue-600 dark:text-blue-400" />
            <div>
              <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Database Manager</h1>
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                Inspect and manage application database
              </p>
            </div>
          </div>
          <button
            onClick={fetchTables}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 flex items-center space-x-2"
          >
            <RefreshCw className="h-4 w-4" />
            <span>Refresh</span>
          </button>
        </div>

        {error && (
          <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md p-4 mb-4">
            <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Tables List */}
        <div className="lg:col-span-1">
          <div className="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Tables</h2>
            <div className="space-y-2">
              {tables.map(table => (
                <button
                  key={table.name}
                  onClick={() => fetchTableData(table.name)}
                  className={`w-full text-left px-3 py-2 rounded-md transition-colors ${
                    selectedTable === table.name
                      ? 'bg-blue-100 dark:bg-blue-900 text-blue-900 dark:text-blue-100'
                      : 'hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300'
                  }`}
                >
                  <div className="font-medium">{table.name}</div>
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    {table.rowCount} rows
                  </div>
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Table Data */}
        <div className="lg:col-span-3">
          <div className="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
            {selectedTable ? (
              <>
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                    {selectedTable} ({filteredData.length} records)
                  </h2>
                  <div className="flex items-center space-x-2">
                    {/* Search */}
                    <div className="relative">
                      <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                      <input
                        type="text"
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        placeholder="Search..."
                        className="pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                      />
                    </div>
                    <button
                      onClick={() => {
                        const tableInfo = tables.find(t => t.name === selectedTable);
                        if (tableInfo) {
                          const emptyRow: DatabaseRecord = {};
                          tableInfo.columns.forEach(col => {
                            emptyRow[col] = '';
                          });
                          setNewRowData(emptyRow);
                        }
                        setShowAddRow(true);
                      }}
                      className="px-3 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 flex items-center space-x-1"
                    >
                      <Plus className="h-4 w-4" />
                      <span>Add</span>
                    </button>
                    <button
                      onClick={handleClearTable}
                      className="px-3 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 flex items-center space-x-1"
                    >
                      <Trash2 className="h-4 w-4" />
                      <span>Clear</span>
                    </button>
                  </div>
                </div>

                {isLoading ? (
                  <div className="flex justify-center py-8">
                    <LoadingSpinner />
                  </div>
                ) : (
                  <div className="overflow-x-auto">
                    <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                      <thead className="bg-gray-50 dark:bg-gray-700">
                        <tr>
                          {(() => {
                            // Get columns from table info or from data
                            const tableInfo = tables.find(t => t.name === selectedTable);
                            const columns = tableInfo?.columns || (tableData.length > 0 ? Object.keys(tableData[0]) : []);
                            return columns.map(column => (
                              <th
                                key={column}
                                className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
                              >
                                {column}
                              </th>
                            ));
                          })()}
                          <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Actions
                          </th>
                        </tr>
                      </thead>
                      <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                        {/* Add new row form */}
                        {showAddRow && (
                          <tr className="bg-green-50 dark:bg-green-900/20">
                            {(() => {
                              const tableInfo = tables.find(t => t.name === selectedTable);
                              const columns = tableInfo?.columns || (tableData.length > 0 ? Object.keys(tableData[0]) : []);
                              return columns.map(column => (
                                <td key={column} className="px-6 py-4 whitespace-nowrap">
                                  <input
                                    type="text"
                                    value={newRowData[column] || ''}
                                    onChange={(e) => setNewRowData({ ...newRowData, [column]: e.target.value })}
                                    className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                    placeholder={column}
                                  />
                                </td>
                              ));
                            })()}
                            <td className="px-6 py-4 whitespace-nowrap text-right">
                              <button
                                onClick={handleAddRow}
                                className="text-green-600 hover:text-green-900 dark:hover:text-green-400 mr-2"
                              >
                                <Save className="h-4 w-4" />
                              </button>
                              <button
                                onClick={() => {
                                  setShowAddRow(false);
                                  setNewRowData({});
                                }}
                                className="text-gray-600 hover:text-gray-900 dark:hover:text-gray-400"
                              >
                                <X className="h-4 w-4" />
                              </button>
                            </td>
                          </tr>
                        )}

                        {/* Data rows */}
                        {filteredData.map((row, index) => (
                          <tr key={index} className="hover:bg-gray-50 dark:hover:bg-gray-700">
                            {Object.entries(row).map(([key, value]) => (
                              <td key={key} className="px-6 py-4 whitespace-nowrap">
                                {editingRow === index ? (
                                  <input
                                    type="text"
                                    value={editedData[key] || ''}
                                    onChange={(e) => setEditedData({ ...editedData, [key]: e.target.value })}
                                    className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                  />
                                ) : (
                                  <span className={`text-sm ${
                                    getColumnType(value) === 'datetime' ? 'font-mono text-xs' :
                                    getColumnType(value) === 'url' ? 'text-blue-600 dark:text-blue-400' :
                                    getColumnType(value) === 'boolean' ? 'font-semibold' :
                                    'text-gray-900 dark:text-white'
                                  }`}>
                                    {value === null ? 'null' : String(value)}
                                  </span>
                                )}
                              </td>
                            ))}
                            <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                              {editingRow === index ? (
                                <>
                                  <button
                                    onClick={() => handleSave(index)}
                                    className="text-green-600 hover:text-green-900 dark:hover:text-green-400 mr-2"
                                  >
                                    <Save className="h-4 w-4" />
                                  </button>
                                  <button
                                    onClick={handleCancel}
                                    className="text-gray-600 hover:text-gray-900 dark:hover:text-gray-400"
                                  >
                                    <X className="h-4 w-4" />
                                  </button>
                                </>
                              ) : (
                                <>
                                  <button
                                    onClick={() => handleEdit(index)}
                                    className="text-indigo-600 hover:text-indigo-900 dark:hover:text-indigo-400 mr-2"
                                  >
                                    <Edit className="h-4 w-4" />
                                  </button>
                                  <button
                                    onClick={() => handleDelete(index)}
                                    className="text-red-600 hover:text-red-900 dark:hover:text-red-400"
                                  >
                                    <Trash2 className="h-4 w-4" />
                                  </button>
                                </>
                              )}
                            </td>
                          </tr>
                        ))}

                        {filteredData.length === 0 && !showAddRow && (
                          <tr>
                            <td
                              colSpan={100}
                              className="px-6 py-12 text-center text-gray-500 dark:text-gray-400"
                            >
                              {searchQuery ? 'No matching records found' : 'No data in this table'}
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </div>
                )}
              </>
            ) : (
              <div className="text-center py-12">
                <DatabaseIcon className="mx-auto h-12 w-12 text-gray-400" />
                <p className="mt-4 text-gray-500 dark:text-gray-400">
                  Select a table to view its contents
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default Database;