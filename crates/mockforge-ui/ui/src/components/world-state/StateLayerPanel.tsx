/**
 * State Layer Panel Component
 *
 * Allows toggling visibility of different state layers
 */

import React from 'react';
import { Card } from '../ui/Card';

export interface StateLayer {
  id: string;
  name: string;
  enabled: boolean;
}

interface StateLayerPanelProps {
  layers: StateLayer[];
  onLayerToggle: (layerId: string, enabled: boolean) => void;
}

export const StateLayerPanel: React.FC<StateLayerPanelProps> = ({
  layers,
  onLayerToggle,
}) => {
  return (
    <Card className="p-4">
      <h3 className="text-lg font-semibold mb-4">State Layers</h3>
      <div className="space-y-2">
        {layers.map((layer) => (
          <label
            key={layer.id}
            className="flex items-center space-x-2 cursor-pointer hover:bg-gray-50 p-2 rounded"
          >
            <input
              type="checkbox"
              checked={layer.enabled}
              onChange={(e) => onLayerToggle(layer.id, e.target.checked)}
              className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
            />
            <span className="text-sm font-medium">{layer.name}</span>
          </label>
        ))}
      </div>
    </Card>
  );
};
