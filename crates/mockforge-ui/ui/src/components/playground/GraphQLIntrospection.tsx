import { logger } from '@/utils/logger';
import React, { useEffect, useState } from 'react';
import { Loader2, RefreshCw, ChevronRight, ChevronDown, Search, Code2 } from 'lucide-react';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Input } from '../ui/input';
import { Badge } from '../ui/Badge';
import { usePlaygroundStore } from '../../stores/usePlaygroundStore';
import { toast } from 'sonner';

/**
 * GraphQL Introspection Component
 *
 * Displays GraphQL schema information with:
 * - Schema explorer (types, queries, mutations, subscriptions)
 * - Type explorer with fields and descriptions
 * - Search functionality
 * - Query builder helper
 */
export function GraphQLIntrospection() {
  const {
    graphQLSchema,
    introspectionLoading,
    introspectionError,
    loadGraphQLIntrospection,
    protocol,
  } = usePlaygroundStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [expandedTypes, setExpandedTypes] = useState<Set<string>>(new Set());
  const [selectedType, setSelectedType] = useState<string | null>(null);

  // Load introspection when GraphQL is selected
  useEffect(() => {
    if (protocol === 'graphql' && !graphQLSchema && !introspectionLoading) {
      loadGraphQLIntrospection();
    }
  }, [protocol, graphQLSchema, introspectionLoading, loadGraphQLIntrospection]);

  // Toggle type expansion
  const toggleType = (typeName: string) => {
    const newExpanded = new Set(expandedTypes);
    if (newExpanded.has(typeName)) {
      newExpanded.delete(typeName);
    } else {
      newExpanded.add(typeName);
    }
    setExpandedTypes(newExpanded);
  };

  // Get types from schema
  const getTypes = () => {
    if (!graphQLSchema?.schema) return [];

    try {
      const schema = graphQLSchema.schema as { types?: Array<{ name?: string; kind?: string }> };
      return schema.types || [];
    } catch {
      return [];
    }
  };

  // Filter types by search query
  const filteredTypes = getTypes().filter((type) => {
    if (!searchQuery) return true;
    const query = searchQuery.toLowerCase();
    return (
      type.name?.toLowerCase().includes(query) ||
      false
    );
  });

  // Get type details
  const getTypeDetails = (typeName: string) => {
    if (!graphQLSchema?.schema) return null;

    try {
      const schema = graphQLSchema.schema as { types?: Array<{ name?: string; [key: string]: unknown }> };
      return schema.types?.find((t) => t.name === typeName);
    } catch {
      return null;
    }
  };

  // Get queries
  const getQueries = () => {
    if (!graphQLSchema?.schema) return [];

    try {
      const schema = graphQLSchema.schema as {
        queryType?: { fields?: Array<{ name?: string; [key: string]: unknown }> };
      };
      return schema.queryType?.fields || [];
    } catch {
      return [];
    }
  };

  // Get mutations
  const getMutations = () => {
    if (!graphQLSchema?.schema) return [];

    try {
      const schema = graphQLSchema.schema as {
        mutationType?: { fields?: Array<{ name?: string; [key: string]: unknown }> };
      };
      return schema.mutationType?.fields || [];
    } catch {
      return [];
    }
  };

  // Get subscriptions
  const getSubscriptions = () => {
    if (!graphQLSchema?.schema) return [];

    try {
      const schema = graphQLSchema.schema as {
        subscriptionType?: { fields?: Array<{ name?: string; [key: string]: unknown }> };
      };
      return schema.subscriptionType?.fields || [];
    } catch {
      return [];
    }
  };

  // Render field type
  const renderFieldType = (type: unknown): string => {
    if (typeof type === 'object' && type !== null) {
      const typeObj = type as { name?: string; kind?: string; ofType?: unknown };
      if (typeObj.name) {
        return typeObj.name;
      }
      if (typeObj.kind === 'LIST') {
        return `[${renderFieldType(typeObj.ofType)}]`;
      }
      if (typeObj.kind === 'NON_NULL') {
        return `${renderFieldType(typeObj.ofType)}!`;
      }
    }
    return 'Unknown';
  };

  // Render field
  const renderField = (field: { name?: string; type?: unknown; description?: string; args?: unknown[] }) => {
    if (!field.name) return null;

    return (
      <div key={field.name} className="ml-4 mb-2 p-2 bg-muted/30 rounded">
        <div className="flex items-start gap-2">
          <code className="text-sm font-semibold text-blue-400">{field.name}</code>
          <span className="text-sm text-muted-foreground">
            : {renderFieldType(field.type)}
          </span>
        </div>
        {field.description && (
          <p className="text-xs text-muted-foreground mt-1 ml-4">{field.description}</p>
        )}
        {field.args && field.args.length > 0 && (
          <div className="mt-2 ml-4">
            <div className="text-xs font-semibold text-muted-foreground mb-1">Arguments:</div>
            {field.args.map((arg: { name?: string; type?: unknown; description?: string }, idx: number) => (
              <div key={idx} className="text-xs ml-2">
                <code className="text-blue-300">{arg.name}</code>
                <span className="text-muted-foreground">: {renderFieldType(arg.type)}</span>
                {arg.description && (
                  <span className="text-muted-foreground ml-2">- {arg.description}</span>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  };

  if (protocol !== 'graphql') {
    return (
      <Card>
        <CardContent className="p-6 text-center text-muted-foreground">
          Switch to GraphQL protocol to view schema introspection
        </CardContent>
      </Card>
    );
  }

  if (introspectionLoading) {
    return (
      <Card>
        <CardContent className="p-6 text-center">
          <Loader2 className="h-6 w-6 animate-spin mx-auto mb-2" />
          <p className="text-sm text-muted-foreground">Loading schema...</p>
        </CardContent>
      </Card>
    );
  }

  if (introspectionError) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-lg font-semibold">Schema Error</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-destructive">{introspectionError}</p>
          <Button
            variant="outline"
            size="sm"
            className="mt-4"
            onClick={() => loadGraphQLIntrospection()}
          >
            <RefreshCw className="h-4 w-4 mr-2" />
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  if (!graphQLSchema) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-lg font-semibold">GraphQL Schema</CardTitle>
        </CardHeader>
        <CardContent>
          <Button
            variant="outline"
            onClick={() => loadGraphQLIntrospection()}
          >
            <RefreshCw className="h-4 w-4 mr-2" />
            Load Schema
          </Button>
        </CardContent>
      </Card>
    );
  }

  const queries = getQueries();
  const mutations = getMutations();
  const subscriptions = getSubscriptions();

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg font-semibold flex items-center gap-2">
            <Code2 className="h-5 w-5" />
            Schema Explorer
          </CardTitle>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => loadGraphQLIntrospection()}
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
        <div className="relative mt-2">
          <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search types..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8"
          />
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-auto space-y-4">
        {/* Queries */}
        {queries.length > 0 && (
          <div>
            <h3 className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Badge variant="outline" className="bg-green-500/10 text-green-600">
                Query
              </Badge>
              Operations
            </h3>
            <div className="space-y-1">
              {queries.map((query) => (
                <div key={query.name} className="p-2 bg-muted/30 rounded">
                  {renderField(query as { name?: string; type?: unknown; description?: string; args?: unknown[] })}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Mutations */}
        {mutations.length > 0 && (
          <div>
            <h3 className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Badge variant="outline" className="bg-blue-500/10 text-blue-600">
                Mutation
              </Badge>
              Operations
            </h3>
            <div className="space-y-1">
              {mutations.map((mutation) => (
                <div key={mutation.name} className="p-2 bg-muted/30 rounded">
                  {renderField(mutation as { name?: string; type?: unknown; description?: string; args?: unknown[] })}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Subscriptions */}
        {subscriptions.length > 0 && (
          <div>
            <h3 className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Badge variant="outline" className="bg-purple-500/10 text-purple-600">
                Subscription
              </Badge>
              Operations
            </h3>
            <div className="space-y-1">
              {subscriptions.map((subscription) => (
                <div key={subscription.name} className="p-2 bg-muted/30 rounded">
                  {renderField(subscription as { name?: string; type?: unknown; description?: string; args?: unknown[] })}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Types */}
        <div>
          <h3 className="text-sm font-semibold mb-2">Types</h3>
          <div className="space-y-1">
            {filteredTypes.map((type) => {
              const typeName = type.name || 'Unknown';
              const isExpanded = expandedTypes.has(typeName);
              const typeDetails = getTypeDetails(typeName);
              const fields = (typeDetails as { fields?: unknown[] })?.fields || [];

              return (
                <div key={typeName} className="border rounded">
                  <button
                    onClick={() => toggleType(typeName)}
                    className="w-full p-2 flex items-center justify-between hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-xs">
                        {type.kind || 'TYPE'}
                      </Badge>
                      <code className="text-sm font-semibold">{typeName}</code>
                    </div>
                    {isExpanded ? (
                      <ChevronDown className="h-4 w-4" />
                    ) : (
                      <ChevronRight className="h-4 w-4" />
                    )}
                  </button>
                  {isExpanded && fields.length > 0 && (
                    <div className="p-2 border-t bg-muted/20">
                      {fields.map((field: { name?: string; [key: string]: unknown }, idx: number) => (
                        <div key={idx}>
                          {renderField(field as { name?: string; type?: unknown; description?: string; args?: unknown[] })}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
