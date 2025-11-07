//! Condition Builder Component
//!
//! Visual UI and code editor for creating conditional transition expressions.
//! Supports both visual builder mode and code editor mode.

import React, { useState } from 'react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/Tabs';
import { Code2, Sliders } from 'lucide-react';

interface ConditionBuilderProps {
  condition: string;
  onUpdate: (condition: string) => void;
  onCancel: () => void;
}

export function ConditionBuilder({ condition, onUpdate, onCancel }: ConditionBuilderProps) {
  const [codeMode, setCodeMode] = useState(true);
  const [codeValue, setCodeValue] = useState(condition);
  const [visualConditions, setVisualConditions] = useState<Array<{
    variable: string;
    operator: string;
    value: string;
    logicalOp?: 'and' | 'or';
  }>>([]);

  const handleCodeUpdate = () => {
    onUpdate(codeValue);
  };

  const handleVisualUpdate = () => {
    // Convert visual conditions to expression
    const expression = visualConditions
      .map((cond, index) => {
        const part = `${cond.variable} ${cond.operator} ${cond.value}`;
        if (index > 0 && cond.logicalOp) {
          return ` ${cond.logicalOp} ${part}`;
        }
        return part;
      })
      .join('');

    onUpdate(expression);
  };

  const addVisualCondition = () => {
    setVisualConditions([
      ...visualConditions,
      {
        variable: '',
        operator: '==',
        value: '',
        logicalOp: visualConditions.length > 0 ? 'and' : undefined,
      },
    ]);
  };

  const removeVisualCondition = (index: number) => {
    setVisualConditions(visualConditions.filter((_, i) => i !== index));
  };

  const updateVisualCondition = (
    index: number,
    field: 'variable' | 'operator' | 'value' | 'logicalOp',
    value: string
  ) => {
    setVisualConditions(
      visualConditions.map((cond, i) =>
        i === index ? { ...cond, [field]: value } : cond
      )
    );
  };

  return (
    <div className="space-y-4">
      <Tabs value={codeMode ? 'code' : 'visual'} onValueChange={(v) => setCodeMode(v === 'code')}>
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="code">
            <Code2 className="h-4 w-4 mr-2" />
            Code
          </TabsTrigger>
          <TabsTrigger value="visual">
            <Sliders className="h-4 w-4 mr-2" />
            Visual
          </TabsTrigger>
        </TabsList>

        {codeMode ? (
          <div className="space-y-2 mt-2">
            <div className="text-xs text-gray-500 dark:text-gray-400 mb-2">
            Enter a JavaScript/TypeScript expression (e.g., "count > 10 && status == 'active'")
          </div>
          <Input
            value={codeValue}
            onChange={(e) => setCodeValue(e.target.value)}
            placeholder="count > 10 && status == 'active'"
            className="font-mono text-sm"
          />
          <div className="flex gap-2">
            <Button onClick={handleCodeUpdate} size="sm" variant="default">
              Apply
            </Button>
            <Button onClick={onCancel} size="sm" variant="outline">
              Cancel
            </Button>
          </div>
        ) : (
          <div className="space-y-2 mt-2">
            <div className="space-y-2">
            {visualConditions.map((cond, index) => (
              <div key={index} className="flex items-center gap-2 p-2 border rounded">
                {index > 0 && (
                  <select
                    value={cond.logicalOp || 'and'}
                    onChange={(e) =>
                      updateVisualCondition(index, 'logicalOp', e.target.value)
                    }
                    className="text-xs border rounded px-2 py-1"
                  >
                    <option value="and">AND</option>
                    <option value="or">OR</option>
                  </select>
                )}
                <Input
                  value={cond.variable}
                  onChange={(e) => updateVisualCondition(index, 'variable', e.target.value)}
                  placeholder="variable"
                  className="text-xs flex-1"
                />
                <select
                  value={cond.operator}
                  onChange={(e) => updateVisualCondition(index, 'operator', e.target.value)}
                  className="text-xs border rounded px-2 py-1"
                >
                  <option value="==">==</option>
                  <option value="!=">!=</option>
                  <option value=">">&gt;</option>
                  <option value="<">&lt;</option>
                  <option value=">=">&gt;=</option>
                  <option value="<=">&lt;=</option>
                </select>
                <Input
                  value={cond.value}
                  onChange={(e) => updateVisualCondition(index, 'value', e.target.value)}
                  placeholder="value"
                  className="text-xs flex-1"
                />
                <Button
                  onClick={() => removeVisualCondition(index)}
                  size="sm"
                  variant="ghost"
                  className="h-6 w-6 p-0"
                >
                  ×
                </Button>
              </div>
            ))}
          </div>
          <Button onClick={addVisualCondition} size="sm" variant="outline" className="w-full">
            Add Condition
          </Button>
          <div className="flex gap-2">
            <Button onClick={handleVisualUpdate} size="sm" variant="default">
              Apply
            </Button>
            <Button onClick={onCancel} size="sm" variant="outline">
              Cancel
            </Button>
          </div>
          </div>
        )}
      </Tabs>
    </div>
  );
}
