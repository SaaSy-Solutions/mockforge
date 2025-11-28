//! Flow Properties Panel Component
//!
//! Panel for editing properties of selected flow steps.

import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { X } from 'lucide-react';
import { Node } from 'react-flow-renderer';
import { ApiCallNodeData } from './ApiCallNode';
import { ConditionNodeData } from './ConditionNode';
import { DelayNodeData } from './DelayNode';
import { LoopNodeData } from './LoopNode';
import { ParallelNodeData } from './ParallelNode';

interface FlowPropertiesPanelProps {
  selectedNode: Node | null;
  onUpdate: (nodeId: string, data: any) => void;
  onClose: () => void;
}

export function FlowPropertiesPanel({
  selectedNode,
  onUpdate,
  onClose,
}: FlowPropertiesPanelProps) {
  if (!selectedNode) {
    return null;
  }

  const nodeType = selectedNode.type || 'apiCall';
  const data = selectedNode.data;

  // API Call Node Properties
  if (nodeType === 'apiCall') {
    const apiData = data as ApiCallNodeData;
    const [name, setName] = useState(apiData.name || '');
    const [method, setMethod] = useState(apiData.method || 'GET');
    const [endpoint, setEndpoint] = useState(apiData.endpoint || '');
    const [expectedStatus, setExpectedStatus] = useState(apiData.expectedStatus?.toString() || '');

    useEffect(() => {
      setName(apiData.name || '');
      setMethod(apiData.method || 'GET');
      setEndpoint(apiData.endpoint || '');
      setExpectedStatus(apiData.expectedStatus?.toString() || '');
    }, [apiData]);

    const handleSave = () => {
      onUpdate(selectedNode.id, {
        ...apiData,
        name,
        method,
        endpoint,
        expectedStatus: expectedStatus ? parseInt(expectedStatus, 10) : undefined,
      });
    };

    return (
      <Card className="w-80">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>API Call Properties</CardTitle>
            <Button variant="ghost" size="sm" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Step name"
            />
          </div>
          <div>
            <Label htmlFor="method">HTTP Method</Label>
            <Select value={method} onValueChange={setMethod}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="GET">GET</SelectItem>
                <SelectItem value="POST">POST</SelectItem>
                <SelectItem value="PUT">PUT</SelectItem>
                <SelectItem value="PATCH">PATCH</SelectItem>
                <SelectItem value="DELETE">DELETE</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div>
            <Label htmlFor="endpoint">Endpoint</Label>
            <Input
              id="endpoint"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="/api/endpoint"
            />
          </div>
          <div>
            <Label htmlFor="expectedStatus">Expected Status Code</Label>
            <Input
              id="expectedStatus"
              type="number"
              value={expectedStatus}
              onChange={(e) => setExpectedStatus(e.target.value)}
              placeholder="200"
            />
          </div>
          <Button onClick={handleSave} className="w-full">
            Save Changes
          </Button>
        </CardContent>
      </Card>
    );
  }

  // Condition Node Properties
  if (nodeType === 'condition') {
    const conditionData = data as ConditionNodeData;
    const [name, setName] = useState(conditionData.name || '');
    const [expression, setExpression] = useState(conditionData.expression || '');

    useEffect(() => {
      setName(conditionData.name || '');
      setExpression(conditionData.expression || '');
    }, [conditionData]);

    const handleSave = () => {
      onUpdate(selectedNode.id, {
        ...conditionData,
        name,
        expression,
      });
    };

    return (
      <Card className="w-80">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Condition Properties</CardTitle>
            <Button variant="ghost" size="sm" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Condition name"
            />
          </div>
          <div>
            <Label htmlFor="expression">Expression</Label>
            <Input
              id="expression"
              value={expression}
              onChange={(e) => setExpression(e.target.value)}
              placeholder="{{response.status}} == 200"
            />
          </div>
          <Button onClick={handleSave} className="w-full">
            Save Changes
          </Button>
        </CardContent>
      </Card>
    );
  }

  // Delay Node Properties
  if (nodeType === 'delay') {
    const delayData = data as DelayNodeData;
    const [name, setName] = useState(delayData.name || '');
    const [delayMs, setDelayMs] = useState(delayData.delayMs?.toString() || '');

    useEffect(() => {
      setName(delayData.name || '');
      setDelayMs(delayData.delayMs?.toString() || '');
    }, [delayData]);

    const handleSave = () => {
      onUpdate(selectedNode.id, {
        ...delayData,
        name,
        delayMs: delayMs ? parseInt(delayMs, 10) : undefined,
      });
    };

    return (
      <Card className="w-80">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Delay Properties</CardTitle>
            <Button variant="ghost" size="sm" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Delay name"
            />
          </div>
          <div>
            <Label htmlFor="delayMs">Delay (milliseconds)</Label>
            <Input
              id="delayMs"
              type="number"
              value={delayMs}
              onChange={(e) => setDelayMs(e.target.value)}
              placeholder="1000"
            />
          </div>
          <Button onClick={handleSave} className="w-full">
            Save Changes
          </Button>
        </CardContent>
      </Card>
    );
  }

  // Loop Node Properties
  if (nodeType === 'loop') {
    const loopData = data as LoopNodeData;
    const [name, setName] = useState(loopData.name || '');
    const [iterations, setIterations] = useState(loopData.iterations?.toString() || '');
    const [condition, setCondition] = useState(loopData.condition || '');

    useEffect(() => {
      setName(loopData.name || '');
      setIterations(loopData.iterations?.toString() || '');
      setCondition(loopData.condition || '');
    }, [loopData]);

    const handleSave = () => {
      onUpdate(selectedNode.id, {
        ...loopData,
        name,
        iterations: iterations ? parseInt(iterations, 10) : undefined,
        condition: condition || undefined,
      });
    };

    return (
      <Card className="w-80">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Loop Properties</CardTitle>
            <Button variant="ghost" size="sm" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Loop name"
            />
          </div>
          <div>
            <Label htmlFor="iterations">Iterations</Label>
            <Input
              id="iterations"
              type="number"
              value={iterations}
              onChange={(e) => setIterations(e.target.value)}
              placeholder="5"
            />
          </div>
          <div>
            <Label htmlFor="condition">Condition</Label>
            <Input
              id="condition"
              value={condition}
              onChange={(e) => setCondition(e.target.value)}
              placeholder="{{items.length}} > 0"
            />
          </div>
          <Button onClick={handleSave} className="w-full">
            Save Changes
          </Button>
        </CardContent>
      </Card>
    );
  }

  // Parallel Node Properties
  if (nodeType === 'parallel') {
    const parallelData = data as ParallelNodeData;
    const [name, setName] = useState(parallelData.name || '');
    const [branches, setBranches] = useState(parallelData.branches?.toString() || '2');

    useEffect(() => {
      setName(parallelData.name || '');
      setBranches(parallelData.branches?.toString() || '2');
    }, [parallelData]);

    const handleSave = () => {
      onUpdate(selectedNode.id, {
        ...parallelData,
        name,
        branches: branches ? parseInt(branches, 10) : 2,
      });
    };

    return (
      <Card className="w-80">
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Parallel Properties</CardTitle>
            <Button variant="ghost" size="sm" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Parallel name"
            />
          </div>
          <div>
            <Label htmlFor="branches">Number of Branches</Label>
            <Input
              id="branches"
              type="number"
              value={branches}
              onChange={(e) => setBranches(e.target.value)}
              placeholder="2"
              min="2"
            />
          </div>
          <Button onClick={handleSave} className="w-full">
            Save Changes
          </Button>
        </CardContent>
      </Card>
    );
  }

  return null;
}

