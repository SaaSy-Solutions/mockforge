/**
 * Composite ApiService class — combines chains, state machines, MockAI, and workspaces
 * into a single class to maintain backward compatibility with `apiService` singleton.
 */
import { ApiService as ChainsApiService } from './chains';
import { StateMachineApiMixin } from './stateMachines';
import { MockAIApiMixin } from './mockai';
import { WorkspacesApiMixin } from './workspaces';

// Build the composite type without using interface-class declaration merging
type ApiServiceType = ChainsApiService & StateMachineApiMixin & MockAIApiMixin & WorkspacesApiMixin;

// Copy prototype methods from each mixin onto the base class
function applyMixins(derivedCtor: { prototype: object }, baseCtors: { prototype: object }[]) {
  baseCtors.forEach(baseCtor => {
    Object.getOwnPropertyNames(baseCtor.prototype).forEach(name => {
      if (name !== 'constructor') {
        Object.defineProperty(
          derivedCtor.prototype,
          name,
          Object.getOwnPropertyDescriptor(baseCtor.prototype, name) || Object.create(null)
        );
      }
    });
  });
}

applyMixins(ChainsApiService, [StateMachineApiMixin, MockAIApiMixin, WorkspacesApiMixin]);

// Cast the enhanced class to the full composite type
const ApiService = ChainsApiService as unknown as new () => ApiServiceType;

export { ApiService };
export type { ApiServiceType };
