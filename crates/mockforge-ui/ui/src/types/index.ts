// ==================== AUTH TYPES ====================

export interface User {
  id: string;
  username: string;
  email: string;
  role: 'admin' | 'user' | 'viewer';
  preferences?: UserPreferences;
}

export interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  user: User | null;
  token: string | null;
  refreshToken: string | null;
}

export interface AuthActions {
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshTokenAction: () => Promise<void>;
  setAuthenticated: (user: User, token: string) => void;
  updateProfile: (userData: User) => Promise<void>;
}

// ==================== FIXTURE TYPES ====================

export interface FixtureInfo {
  id: string;
  name: string;
  path: string;
  method?: string;
  description?: string;
  createdAt: string;
  updatedAt: string;
  tags?: string[];
  content?: string | unknown;
  version?: string;
  size_bytes?: number;
  last_modified?: string;
  route_path?: string;
  protocol?: string;
  saved_at?: string;
  fingerprint?: string;
  metadata?: Record<string, unknown>;
  file_size?: number;
  file_path?: string;
  size?: number;
  created_at?: string;
  modified_at?: string;
}

export interface DiffChange {
  type: 'add' | 'remove' | 'modify';
  path: string;
  oldValue?: unknown;
  newValue?: unknown;
  line_number?: number;
  content?: string;
  old_content?: string;
}

export interface FixtureDiff {
  id: string;
  name: string;
  changes: DiffChange[];
  timestamp: string;
  new_content?: string;
}

// ==================== LOG TYPES ====================

export interface RequestLog {
  id: string;
  timestamp: string;
  method: string;
  path: string;
  status_code: number;
  response_time_ms: number;
  client_ip?: string;
  user_agent?: string;
  headers: Record<string, string>;
  response_size_bytes: number;
  error_message?: string;
}

export interface LogFilter {
  status_code?: number[] | number;
  method?: string[] | string;
  path?: string;
  path_pattern?: string;
  level?: string[] | string;
  date_range?: {
    start: string;
    end: string;
  };
  hours_ago?: number;
  limit?: number;
  offset?: number;
}

// ==================== METRICS TYPES ====================

export interface MetricsData {
  requests_by_endpoint: Record<string, number>;
  response_time_percentiles: Record<string, number>;
  error_rate_by_endpoint: Record<string, number>;
  memory_usage_over_time: Array<[string, number]>;
  cpu_usage_over_time: Array<[string, number]>;
}

export interface ValidationSettings {
  mode: string;
  aggregate_errors: boolean;
  validate_responses: boolean;
  overrides: Record<string, string>;
}

export interface LatencyProfile {
  name: string;
  base_ms: number;
  jitter_ms: number;
  tag_overrides: Record<string, number>;
}

export interface FaultConfig {
  enabled: boolean;
  failure_rate: number;
  status_codes: number[];
  active_failures: number;
}

export interface ProxyConfig {
  enabled: boolean;
  upstream_url?: string;
  timeout_seconds: number;
  requests_proxied: number;
}

export interface ServerInfo {
  version: string;
  build_time: string;
  git_sha: string;
  http_server?: string;
  ws_server?: string;
  grpc_server?: string;
  graphql_server?: string;
  api_enabled: boolean;
  admin_port: number;
}

export interface DashboardSystemInfo {
  os: string;
  arch: string;
  uptime: number;
  memory_usage: number;
}

export interface SimpleMetricsData {
  total_requests: number;
  active_requests: number;
  average_response_time: number;
  error_rate: number;
}

export interface ServerStatus {
  server_type: string;
  address?: string;
  running: boolean;
  start_time?: string;
  uptime_seconds?: number;
  active_connections: number;
  total_requests: number;
}

export interface SystemInfo {
  version: string;
  uptime_seconds: number;
  memory_usage_mb: number;
  cpu_usage_percent: number;
  active_threads: number;
  total_routes: number;
  total_fixtures: number;
}

export interface DashboardData {
  server_info: ServerInfo;
  system_info: DashboardSystemInfo;
  metrics: SimpleMetricsData;
  servers: ServerStatus[];
  recent_logs: RequestLog[];
  system: SystemInfo;
}

export interface LatencyMetrics {
  avg_response_time: number;
  min_response_time: number;
  max_response_time: number;
  p50_response_time: number;
  p95_response_time: number;
  p99_response_time: number;
  total_requests: number;
  service?: string;
  route?: string;
  histogram?: HistogramBucket[];
  p50?: number;
  p95?: number;
  p99?: number;
}

export interface FailureMetrics {
  total_errors: number;
  error_rate_percentage: number;
  error_by_status_code: Record<number, number>;
  service?: string;
  total_requests?: number;
  success_count?: number;
  failure_count?: number;
  status_codes?: Record<number, number>;
  error_rate?: number;
}

export interface HistogramBucket {
  le: number; // Less than or equal
  count: number;
  range?: string;
}

// ==================== PREFERENCES TYPES ====================

export interface UserPreferences {
  theme: UIThemePreferences;
  logs: LogPreferences;
  notifications: NotificationPreferences;
  search: SearchPreferences;
  ui: UIBehaviorPreferences;
}

export interface PreferencesState {
  preferences: UserPreferences;
  loading: boolean;
  error: string | null;
}

export interface PreferencesActions {
  updatePreferences: (preferences: Partial<UserPreferences>) => void;
  updateTheme: (themeUpdates: Partial<UIThemePreferences>) => void;
  updateLogs: (logsUpdates: Partial<LogPreferences>) => void;
  updateNotifications: (notificationsUpdates: Partial<NotificationPreferences>) => void;
  updateSearch: (searchUpdates: Partial<SearchPreferences>) => void;
  updateUI: (uiUpdates: Partial<UIBehaviorPreferences>) => void;
  loadPreferences: () => Promise<void>;
  savePreferences: () => Promise<void>;
  resetToDefaults: () => void;
}

export interface PreferencesStore extends PreferencesState, PreferencesActions {}

export interface UIThemePreferences {
  theme: 'light' | 'dark' | 'system';
  accentColor: string;
  fontSize: 'small' | 'medium' | 'large';
  highContrast: boolean;
}

export interface LogPreferences {
  autoScroll: boolean;
  pauseOnError: boolean;
  showTimestamps: boolean;
  compactView: boolean;
  defaultTimeRange: number;
  itemsPerPage: number;
}

export interface NotificationPreferences {
  enableSounds: boolean;
  showToasts: boolean;
  toastDuration: number;
  notifyOnErrors: boolean;
  notifyOnSuccess: boolean;
}

export interface SearchPreferences {
  defaultScope: 'all' | 'current' | 'logs' | 'services';
  searchHistory: string[];
  maxHistoryItems: number;
  caseSensitive: boolean;
  regexEnabled: boolean;
}

export interface UIBehaviorPreferences {
  sidebarCollapsed: boolean;
  defaultPage: string;
  confirmDelete: boolean;
  autoSave: boolean;
  keyboardShortcuts: boolean;
  serverTableDensity: 'compact' | 'normal' | 'comfortable';
}

// ==================== SERVICE TYPES ====================

export interface ServiceInfo {
  id: string;
  name: string;
  description?: string;
  baseUrl: string;
  enabled: boolean;
  routes: RouteInfo[];
  tags?: string[];
  createdAt: string;
  updatedAt: string;
}

export interface RouteInfo {
  id: string;
  path: string;
  method: string;
  description?: string;
  statusCode: number;
  responseBody?: unknown;
  responseHeaders?: Record<string, string>;
  delay?: number;
  tags?: string[];
  enabled?: boolean;
  request_count?: number;
  latency_ms?: number;
  error_count?: number;
  priority?: number;
  has_fixtures?: boolean;
  service_id?: string;
}

// ==================== API SERVICE TYPES ====================

export interface EnvironmentListResponse {
  environments: Environment[];
  total: number;
}

export interface EnvironmentVariablesResponse {
  variables: EnvironmentVariable[];
}

export interface CreateEnvironmentResponse {
  id: string;
  message: string;
}

// Legacy autocomplete response (use AutocompleteResponse below instead)
export interface LegacyAutocompleteResponse {
  suggestions: string[];
  total: number;
}

// Request types
export interface CreateEnvironmentRequest {
  name: string;
  description?: string;
  variables?: EnvironmentVariable[];
}

export interface UpdateEnvironmentRequest {
  name?: string;
  description?: string;
  variables?: EnvironmentVariable[];
}

export interface SetVariableRequest {
  key: string;
  value: string;
  encrypted?: boolean;
}

export interface AutocompleteSuggestion {
  text: string;
  display_text?: string;
  kind?: string;
  documentation?: string;
}

export interface AutocompleteResponse {
  suggestions: AutocompleteSuggestion[];
  start_position: number;
  end_position: number;
}

export interface AutocompleteRequest {
  input: string;
  cursor_position: number;
  context?: string;
}

export interface EnvironmentColor {
  hex: string;
  name: string;
}

export interface Environment {
  id: string;
  name: string;
  description?: string;
  color?: EnvironmentColor;
  createdAt: string;
  updatedAt: string;
  variables: EnvironmentVariable[];
}

export interface EnvironmentSummary {
  id: string;
  name: string;
  description?: string;
  variable_count: number;
  is_global?: boolean;
  active?: boolean;
  color?: EnvironmentColor;
}

export interface EnvironmentVariable {
  id: string;
  key: string;
  value: string;
  encrypted: boolean;
  createdAt: string;
}

// ==================== UI COMPONENT TYPES ====================


export interface AlertProps {
  type: 'success' | 'error' | 'warning' | 'info';
  title?: string;
  children?: React.ReactNode;
}

export interface CardProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export interface ButtonProps {
  onClick?: () => void;
  disabled?: boolean;
  variant?: 'primary' | 'secondary' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  children: React.ReactNode;
  className?: string;
}

export interface BadgeProps {
  variant?: 'success' | 'warning' | 'error' | 'info';
  children: React.ReactNode;
  className?: string;
}

export interface TableProps {
  columns: TableColumn[];
  data: unknown[];
  className?: string;
}

export interface TableColumn {
  key: string;
  label: string;
  render?: (value: unknown, row: unknown) => React.ReactNode;
}

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
}

export interface InputProps {
  type?: 'text' | 'number' | 'email' | 'password';
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
}

export interface TextareaProps {
  value: string;
  onChange: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  placeholder?: string;
  disabled?: boolean;
  rows?: number;
  className?: string;
}

export interface LabelProps {
  children: React.ReactNode;
  className?: string;
  required?: boolean;
}

export interface SelectProps {
  value: string;
  onValueChange: (value: string) => void;
  children: React.ReactNode;
  disabled?: boolean;
  className?: string;
}

export interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: React.ReactNode;
}

export interface DialogContentProps {
  className?: string;
  children: React.ReactNode;
}

export interface DialogDescriptionProps {
  children: React.ReactNode;
}

export interface DialogFooterProps {
  children: React.ReactNode;
}

export interface DialogHeaderProps {
  children: React.ReactNode;
}

export interface DialogTitleProps {
  children: React.ReactNode;
}

export interface DialogTriggerProps {
  children: React.ReactNode;
  className?: string;
}

export interface TabsProps {
  value: string;
  onValueChange: (value: string) => void;
  children: React.ReactNode;
  className?: string;
}

export interface TabsContentProps {
  value: string;
  children: React.ReactNode;
}

export interface TabsListProps {
  children: React.ReactNode;
  className?: string;
}

export interface TabsTriggerProps {
  value: string;
  children: React.ReactNode;
  className?: string;
}

// ==================== RESPONSE HISTORY TYPES ====================

export interface ResponseHistoryEntry {
  executed_at: string;
  request_method: string;
  request_path: string;
  request_headers: Record<string, string>;
  request_body?: string;
  response_status_code: number;
  response_headers: Record<string, string>;
  response_body?: string;
  response_time_ms: number;
  response_size_bytes: number;
  error_message?: string;
}

export interface RequestHistoryResponse {
  history: ResponseHistoryEntry[];
  total: number;
}

// ==================== SPECIFIC PAGE TYPES ====================

// ImportPage types
export type ImportFormat = 'json' | 'yaml' | 'postman' | 'history';

export interface FileUploadProps {
  onFileSelect: (file: File, format: ImportFormat) => void;
  format: ImportFormat;
}

export interface PageHeaderProps {
  title: string;
  description?: string;
  icon?: React.ReactNode;
  actions?: React.ReactNode;
}

// MetricsPage types
export interface MetricData {
  label: string;
  value: number;
  color: string;
}

export interface SimpleBarChartProps {
  data: MetricData[];
  title: string;
}

// LogsPage types
export interface FilteredLog extends RequestLog {
  index: number;
}

export interface LogEntry {
  timestamp: string;
  status: number;
  method: string;
  url: string;
  responseTime: number;
  size: number;
}

export interface ColumnType {
  key: string;
  label: string;
  sortable?: boolean;
  render?: (value: unknown, row: LogEntry) => React.ReactNode;
}

// PluginsPage types (minimal placeholder, can be extended)
export interface PluginInfo {
  id: string;
  name: string;
  type: 'authentication' | 'template' | 'response';
  enabled: boolean;
  description?: string;
  version?: string;
}

// WorkspacePage types
export interface Workspace {
  id: string;
  name: string;
  description?: string;
  createdAt: string;
  updatedAt: string;
  folders: Folder[];
  requests: Request[];
}

export interface WorkspaceSummary {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
  is_active: boolean;
  config_count: number;
  service_count: number;
  request_count?: number;
  folder_count?: number;
}

export interface Folder {
  id: string;
  name: string;
  description?: string;
  requests: Request[];
  subfolders: Folder[];
}

export interface Request {
  id: string;
  name: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  path: string;
  status_code: number;
  response_body?: string;
}

export interface ImportData {
  format: ImportFormat;
  data: string;
  workspaceId?: string;
}

export interface ChainValidationResponse {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// ==================== CHAIN TYPES ====================

// Chain Summary
export interface ChainSummary {
  id: string;
  name: string;
  description?: string;
  tags: string[];
  enabled: boolean;
  linkCount: number;
}

// Chain List Response
export interface ChainListResponse {
  chains: ChainSummary[];
  total: number;
}

// Chain Execution Response
export interface ChainExecutionResponse {
  chainId: string;
  status: string;
  totalDurationMs: number;
  requestResults?: unknown;
  errorMessage?: string;
}

// Chain Creation Response
export interface ChainCreationResponse {
  id: string;
  message: string;
}

// Chain Configuration
export interface ChainConfig {
  enabled: boolean;
  maxChainLength: number;
  globalTimeoutSecs: number;
  enableParallelExecution: boolean;
}

// Chain Request
export interface ChainRequest {
  id: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: unknown;
  dependsOn: string[];
  timeoutSecs?: number;
  expectedStatus?: number[];
}

// Chain Link
export interface ChainLink {
  request: ChainRequest;
  extract: Record<string, string>;
  storeAs?: string;
}

// Chain Definition
export interface ChainDefinition {
  id: string;
  name: string;
  description?: string;
  config: ChainConfig;
  links: ChainLink[];
  variables: Record<string, unknown>;
  tags: string[];
}

// ==================== SYNC TYPES ====================

export interface SyncConfig {
  enabled: boolean;
  target_directory?: string;
  directory_structure: SyncDirectoryStructure;
  sync_direction: SyncDirection;
  include_metadata: boolean;
  realtime_monitoring: boolean;
  filename_pattern: string;
  exclude_pattern?: string;
  force_overwrite: boolean;
}

export type SyncDirectoryStructure = 'Flat' | 'Nested' | 'Grouped';

export type SyncDirection = 'Manual' | 'WorkspaceToDirectory' | 'Bidirectional';

export interface SyncStatus {
  workspace_id: string;
  enabled: boolean;
  target_directory?: string;
  sync_direction: SyncDirection;
  realtime_monitoring: boolean;
  last_sync?: string;
  status: string;
}

export interface SyncChange {
  change_type: string;
  path: string;
  description: string;
  requires_confirmation: boolean;
}

export interface ConfigureSyncRequest {
  target_directory: string;
  sync_direction: SyncDirection;
  realtime_monitoring: boolean;
  directory_structure?: SyncDirectoryStructure;
  filename_pattern?: string;
}

export interface ConfirmSyncChangesRequest {
  workspace_id: string;
  changes: SyncChange[];
  apply_all: boolean;
}

// ==================== IMPORT TYPES ====================

export interface ImportRequest {
  content: string;
  filename?: string;
  environment?: string;
  base_url?: string;
}

export interface ImportRoute {
  method: string;
  path: string;
  name?: string;
  description?: string;
  headers?: Record<string, string>;
  body?: string;
  status_code?: number;
  response?: {
    status: number;
    headers: Record<string, string>;
    body: string;
  };
}

// Export for use in other files
export type { ImportRoute as ImportedRoute };

export interface ImportResponse {
  success: boolean;
  routes?: ImportRoute[];
  variables?: Record<string, string>;
  warnings: string[];
  error?: string;
}

export interface ImportToWorkspaceRequest {
  format: string;
  data: string;
  folder_id?: string;
  create_folders?: boolean;
  selected_routes?: number[];
}

export interface ImportHistoryEntry {
  id: string;
  format: string;
  timestamp: string;
  routeCount: number;
  routes_count?: number; // Alias for routeCount
  success: boolean;
  filename?: string;
  variables_count?: number;
  warnings_count?: number;
  environment?: string;
  base_url?: string;
  error_message?: string;
}

export interface ImportHistoryResponse {
  imports: ImportHistoryEntry[];
  total: number;
}

// ==================== ENCRYPTION TYPES ====================

export interface EncryptionStatus {
  enabled: boolean;
  algorithm?: string;
  key_id?: string;
  last_rotated?: string;
  backupKey?: string;
  masterKeySet?: boolean;
  workspaceKeySet?: boolean;
}

export interface AutoEncryptionConfig {
  enabled: boolean;
  patterns?: string[];
  algorithm?: string;
  sensitiveHeaders?: string[];
  sensitiveFields?: string[];
  sensitiveEnvVars?: string[];
  customPatterns?: string[];
}

export interface SecurityCheckResult {
  passed: boolean;
  checks: Array<{
    name: string;
    passed: boolean;
    message?: string;
  }>;
  isSecure?: boolean;
  warnings?: string[];
  errors?: string[];
  recommendations?: string[];
}

export interface SecurityWarning {
  severity: 'low' | 'medium' | 'high';
  message: string;
  details?: string;
}

// ==================== WORKSPACE API TYPES ====================

export interface CreateWorkspaceRequest {
  name: string;
  description?: string;
}

export interface CreateFolderRequest {
  name: string;
  description?: string;
  parent_id?: string;
}

export interface CreateRequestRequest {
  name: string;
  description?: string;
  method: string;
  path: string;
  status_code?: number;
  response_body?: string;
  folder_id?: string;
}

export interface FolderSummary {
  id: string;
  name: string;
  description?: string;
  parent_id?: string;
  subfolder_count: number;
  request_count: number;
  created_at: string;
  updated_at: string;
}

export interface RequestSummary {
  id: string;
  name: string;
  description?: string;
  method: string;
  path: string;
  status_code: number;
  created_at: string;
  updated_at: string;
}

export interface WorkspaceListResponse {
  workspaces: WorkspaceSummary[];
}

export interface WorkspaceResponse {
  summary: WorkspaceSummary;
  folders: FolderSummary[];
  requests: RequestSummary[];
}

export interface CreateWorkspaceResponse {
  id: string;
  message: string;
}

export interface FolderResponse {
  summary: FolderSummary;
  subfolders: FolderSummary[];
  requests: RequestSummary[];
}

export interface CreateFolderResponse {
  id: string;
  message: string;
}

export interface CreateRequestResponse {
  id: string;
  message: string;
}

export interface ExecuteRequestRequest {
  variables?: Record<string, unknown>;
}

export interface ExecuteRequestResponse {
  executed_at: string;
  request_method: string;
  request_path: string;
  request_headers: Record<string, string>;
  request_body?: string;
  response_status_code: number;
  response_headers: Record<string, string>;
  response_body?: string;
  response_time_ms: number;
  response_size_bytes: number;
  error_message?: string;
}

// ==================== ADMIN API TYPES ====================

export interface HealthCheck {
  status: string;
  services: Record<string, string>;
  last_check: string;
  issues: string[];
}

export interface RestartStatus {
  restarting: boolean;
  progress: number;
  message: string;
}

export interface ConfigurationState {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface SmokeTestResult {
  test_name: string;
  passed: boolean;
  response_time_ms?: number;
  error_message?: string;
}

export interface SmokeTestContext {
  suite_name: string;
  total_tests: number;
  passed_tests: number;
  failed_tests: number;
  start_time: string;
  end_time?: string;
}

export interface FileContentRequest {
  path: string;
  encoding?: string;
}

export interface FileContentResponse {
  content: string;
  encoding: string;
  size: number;
}

export interface SaveFileRequest {
  path: string;
  content: string;
  encoding?: string;
}
