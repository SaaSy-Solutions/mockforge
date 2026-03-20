/**
 * Barrel re-export — maintains the existing API surface from the monolithic api.ts.
 *
 * All existing imports like `import { apiService } from '@/services/api'` continue
 * to work because this file re-exports everything that the old api.ts exported.
 */
import { logger } from '@/utils/logger';

// Import all service classes from domain modules
import { ApiService } from './apiService';
import { ImportApiService } from './imports';
import { FixturesApiService } from './fixtures';
import { DashboardApiService } from './dashboard';
import { ServerApiService } from './server';
import { RoutesApiService } from './routes';
import { LogsApiService } from './logs';
import { MetricsApiService } from './metrics';
import { ConfigApiService } from './config';
import { ValidationApiService } from './validation';
import { EnvApiService } from './env';
import { FilesApiService } from './files';
import { SmokeTestsApiService } from './smokeTests';
import { ChaosApiService } from './chaos';
import { TimeTravelApiService } from './timeTravel';
import { RealityApiService } from './reality';
import { ConsistencyApiService } from './consistency';
import { SnapshotsApiService } from './snapshots';
import { PluginsApiService } from './plugins';
import { VerificationApiService } from './verification';
import { ContractDiffApiService } from './contractDiff';
import { ProxyApiService } from './proxy';

// Re-export types from types/index.ts (backwards compat)
export type { RequestLog, MetricsData, ValidationSettings, LatencyProfile, FaultConfig, ProxyConfig, DashboardData } from '../../types';
export type { HealthCheck, RestartStatus, SmokeTestResult, SmokeTestContext } from '../../types';
export type { ImportRequest, ImportResponse, ImportHistoryResponse, ImportHistoryEntry } from '../../types';
export type { FixtureInfo } from '../../types';
export type { WorkspaceListResponse, WorkspaceResponse, CreateWorkspaceRequest, CreateWorkspaceResponse } from '../../types';
export type { FolderResponse, CreateFolderRequest, CreateFolderResponse } from '../../types';
export type { CreateRequestRequest, CreateRequestResponse, ExecuteRequestRequest, ExecuteRequestResponse } from '../../types';

// Re-export contract diff types
export type {
  CapturedRequest,
  ContractDiffResult,
  Mismatch,
  Recommendation,
  CorrectionProposal,
  CaptureStatistics,
  AnalyzeRequestPayload,
} from './contractDiff';

// Re-export proxy types
export type {
  ProxyRule,
  ProxyRuleRequest,
  ProxyRulesResponse,
  ProxyInspectResponse,
} from './proxy';

// Instantiate singletons
export const apiService = new ApiService();
export const importApi = new ImportApiService();
export const fixturesApi = new FixturesApiService();
export const proxyApi = new ProxyApiService();

// Admin API services
export const dashboardApi = new DashboardApiService();
export const serverApi = new ServerApiService();
export const routesApi = new RoutesApiService();
export const logsApi = new LogsApiService();
export const metricsApi = new MetricsApiService();
export const configApi = new ConfigApiService();
export const validationApi = new ValidationApiService();
export const envApi = new EnvApiService();
export const filesApi = new FilesApiService();
export const smokeTestsApi = new SmokeTestsApiService();
export const pluginsApi = new PluginsApiService();
export const chaosApi = new ChaosApiService();
export const timeTravelApi = new TimeTravelApiService();
export const realityApi = new RealityApiService();
export const consistencyApi = new ConsistencyApiService();
export const snapshotsApi = new SnapshotsApiService();
export const verificationApi = new VerificationApiService();
export const contractDiffApi = new ContractDiffApiService();

// Debug: Log to verify services are created
logger.info('API Services initialized', {
  apiService: !!apiService,
  importApi: !!importApi,
  fixturesApi: !!fixturesApi,
  fixturesApiGetFixtures: typeof fixturesApi?.getFixtures,
  dashboardApi: !!dashboardApi,
  serverApi: !!serverApi,
  routesApi: !!routesApi,
  logsApi: !!logsApi,
  metricsApi: !!metricsApi,
  configApi: !!configApi,
  validationApi: !!validationApi,
  envApi: !!envApi,
  filesApi: !!filesApi,
  smokeTestsApi: !!smokeTestsApi,
  pluginsApi: !!pluginsApi,
  chaosApi: !!chaosApi,
  timeTravelApi: !!timeTravelApi,
});
