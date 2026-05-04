import React from 'react';
import { cn } from '../../../utils/cn';

interface TableColumn<T = Record<string, unknown>> {
  key: string;
  label: string;
  render?: (value: unknown, row: T) => React.ReactNode;
  sortable?: boolean;
  width?: string;
}

interface TableProps<T = Record<string, unknown>> {
  columns: TableColumn<T>[];
  data: T[];
  className?: string;
  onRowClick?: (row: T) => void;
}

export function Table<T = Record<string, unknown>>({ columns, data, className, onRowClick }: TableProps<T>) {
  return (
    <div className={cn('overflow-x-auto', className)}>
      <table className="min-w-full divide-y divide-border">
        <thead className="bg-muted">
          <tr>
            {columns.map((column, index) => (
              <th
                key={column.key || index}
                className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider"
                style={{ width: column.width }}
              >
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="bg-card divide-y divide-border">
          {data.map((row, rowIndex) => (
            <tr
              key={rowIndex}
              className={cn(
                'hover:bg-accent hover:text-accent-foreground transition-colors duration-150',
                onRowClick && 'cursor-pointer'
              )}
              onClick={() => onRowClick?.(row)}
            >
              {columns.map((column, colIndex) => (
                <td
                  key={column.key || colIndex}
                  className="px-6 py-4 whitespace-nowrap text-sm text-foreground"
                >
                  {column.render
                    ? column.render((row as Record<string, unknown>)[column.key], row)
                    : String((row as Record<string, unknown>)[column.key] ?? '')
                  }
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
