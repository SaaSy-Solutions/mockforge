/**
 * Data-Driven Suite Wizard.
 *
 * Three-step flow that takes a user from "I have a CSV/JSON file of
 * test vectors and an OpenAPI spec" to "I have a runnable
 * data_driven test_suite":
 *
 *   1. Source — target URL + OpenAPI spec
 *   2. Test data — local file → presigned PUT to Tigris → GET URL
 *   3. Run config — name, vus, duration → POST /test-suites
 *
 * The Tigris upload happens browser-side: registry hands out a
 * presigned PUT URL, the browser uploads directly. The registry only
 * sees the upload-URL request and the eventual suite-create call —
 * never the file bytes.
 *
 * After creation, the page shows the suite ID + a "Trigger run" button
 * that hits the existing `triggerRun` endpoint.
 */
import React, { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Upload, CheckCircle2, AlertCircle, Play, ChevronLeft, ChevronRight } from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import { cloudRunsApi, type DataDrivenUploadUrlResponse } from '../services/api/cloudRuns';
import { cloudTestRunsApi, type TestSuite } from '../services/api/cloudTestRuns';

type Step = 'source' | 'data' | 'config' | 'created';

export const CloudDataDrivenWizardPage: React.FC = () => {
  if (!isCloudMode()) {
    return (
      <div className="p-6 max-w-4xl mx-auto">
        <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
          The data-driven suite wizard only works in cloud mode (it uploads
          test-vector files to Tigris and creates suites in the registry).
        </div>
      </div>
    );
  }
  return <Wizard />;
};

const Wizard: React.FC = () => {
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);

  const [step, setStep] = useState<Step>('source');

  // Step 1
  const [targetUrl, setTargetUrl] = useState('https://');
  const [spec, setSpec] = useState('');

  // Step 2
  const [dataFile, setDataFile] = useState<File | null>(null);
  const [uploadResult, setUploadResult] = useState<DataDrivenUploadUrlResponse | null>(null);
  const [uploadError, setUploadError] = useState<string | null>(null);

  // Step 3
  const [name, setName] = useState('');
  const [vus, setVus] = useState(10);
  const [duration, setDuration] = useState('30s');
  const [scenario, setScenario] = useState('constant');
  const [distribution, setDistribution] = useState('unique-per-vu');
  const [mappings, setMappings] = useState('');

  // Outcome
  const [createdSuite, setCreatedSuite] = useState<TestSuite | null>(null);

  const uploadMutation = useMutation({
    mutationFn: async (file: File) => {
      const ext = file.name.split('.').pop()?.toLowerCase() ?? 'csv';
      const urls = await cloudRunsApi.requestDataDrivenUploadUrl({
        extension: ext,
      });
      // Browser uploads the file bytes directly to Tigris via the
      // presigned PUT. Registry doesn't see the body.
      const putRes = await fetch(urls.upload_url, {
        method: 'PUT',
        body: file,
        // Tigris (S3-compatible) expects the body to match what was
        // signed; presigned PUT URLs from aws-sdk-s3 don't bind a
        // Content-Type by default, so the browser's auto-detected one
        // is fine.
      });
      if (!putRes.ok) {
        throw new Error(
          `Tigris rejected the upload (HTTP ${putRes.status}). The presigned URL may have expired — retry.`,
        );
      }
      return urls;
    },
    onSuccess: (urls) => {
      setUploadResult(urls);
      setUploadError(null);
      setStep('config');
    },
    onError: (err: Error) => setUploadError(err.message),
  });

  const createMutation = useMutation({
    mutationFn: async (workspaceId: string) => {
      if (!uploadResult || !dataFile) {
        throw new Error('upload result missing — go back and re-upload');
      }
      const ext = dataFile.name.split('.').pop()?.toLowerCase();
      const dataFormat = ext === 'json' ? 'json' : 'csv';
      const config: Record<string, unknown> = {
        use_cloud_api: true,
        target_url: targetUrl.trim(),
        spec,
        spec_format: spec.trim().startsWith('{') ? 'json' : 'yaml',
        data_url: uploadResult.data_url,
        data_format: dataFormat,
        data_distribution: distribution,
        duration,
        vus,
        scenario,
      };
      if (mappings.trim()) {
        config.data_mappings = mappings.trim();
      }
      return cloudTestRunsApi.createSuite(workspaceId, {
        name: name.trim(),
        kind: 'data_driven',
        config,
      });
    },
    onSuccess: (suite) => {
      setCreatedSuite(suite);
      setStep('created');
    },
  });

  const triggerMutation = useMutation({
    mutationFn: (suiteId: string) =>
      cloudTestRunsApi.triggerRun(suiteId, { triggered_by: 'manual' }),
  });

  if (!activeWorkspace) {
    return (
      <div className="p-6 max-w-4xl mx-auto">
        <div className="bg-amber-50 dark:bg-amber-900/20 text-amber-800 dark:text-amber-300 p-4 rounded-lg">
          Pick an active workspace before creating a suite.
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
          Data-Driven Suite Wizard
        </h1>
        <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
          Upload CSV/JSON test vectors, point at an OpenAPI spec + target URL,
          and ship a ready-to-run <code className="font-mono text-xs">data_driven</code>{' '}
          test suite.
        </p>
      </div>

      <Stepper current={step} />

      <div className="mt-6">
        {step === 'source' && (
          <SourceStep
            targetUrl={targetUrl}
            setTargetUrl={setTargetUrl}
            spec={spec}
            setSpec={setSpec}
            onNext={() => setStep('data')}
          />
        )}
        {step === 'data' && (
          <DataStep
            dataFile={dataFile}
            setDataFile={setDataFile}
            isUploading={uploadMutation.isPending}
            error={uploadError}
            onBack={() => setStep('source')}
            onUpload={() => {
              if (dataFile) {
                setUploadError(null);
                uploadMutation.mutate(dataFile);
              }
            }}
          />
        )}
        {step === 'config' && uploadResult && (
          <ConfigStep
            name={name}
            setName={setName}
            vus={vus}
            setVus={setVus}
            duration={duration}
            setDuration={setDuration}
            scenario={scenario}
            setScenario={setScenario}
            distribution={distribution}
            setDistribution={setDistribution}
            mappings={mappings}
            setMappings={setMappings}
            isCreating={createMutation.isPending}
            error={createMutation.error?.message ?? null}
            uploadResult={uploadResult}
            onBack={() => setStep('data')}
            onCreate={() => createMutation.mutate(activeWorkspace.id)}
          />
        )}
        {step === 'created' && createdSuite && (
          <CreatedStep
            suite={createdSuite}
            onTrigger={() => triggerMutation.mutate(createdSuite.id)}
            triggerInFlight={triggerMutation.isPending}
            triggerError={triggerMutation.error?.message ?? null}
            triggeredRun={triggerMutation.data ?? null}
          />
        )}
      </div>
    </div>
  );
};

const Stepper: React.FC<{ current: Step }> = ({ current }) => {
  const steps: { id: Step; label: string }[] = [
    { id: 'source', label: 'Source' },
    { id: 'data', label: 'Test data' },
    { id: 'config', label: 'Run config' },
    { id: 'created', label: 'Done' },
  ];
  const currentIdx = steps.findIndex((s) => s.id === current);
  return (
    <ol className="flex items-center gap-4">
      {steps.map((s, i) => {
        const done = i < currentIdx;
        const active = i === currentIdx;
        return (
          <li key={s.id} className="flex items-center gap-2">
            <span
              className={`flex items-center justify-center w-7 h-7 rounded-full text-xs font-semibold ${
                active
                  ? 'bg-blue-600 text-white'
                  : done
                    ? 'bg-green-600 text-white'
                    : 'bg-gray-200 text-gray-500 dark:bg-gray-800 dark:text-gray-400'
              }`}
            >
              {done ? <CheckCircle2 className="w-4 h-4" /> : i + 1}
            </span>
            <span
              className={`text-sm ${
                active
                  ? 'font-semibold text-gray-900 dark:text-gray-100'
                  : 'text-gray-500 dark:text-gray-400'
              }`}
            >
              {s.label}
            </span>
            {i < steps.length - 1 && (
              <span className="text-gray-300 dark:text-gray-700 mx-2">›</span>
            )}
          </li>
        );
      })}
    </ol>
  );
};

interface SourceStepProps {
  targetUrl: string;
  setTargetUrl: (s: string) => void;
  spec: string;
  setSpec: (s: string) => void;
  onNext: () => void;
}

const SourceStep: React.FC<SourceStepProps> = ({
  targetUrl,
  setTargetUrl,
  spec,
  setSpec,
  onNext,
}) => {
  const canContinue = targetUrl.trim().startsWith('http') && spec.trim().length > 0;
  return (
    <div className="space-y-4 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
      <div>
        <label
          htmlFor="target-url"
          className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          Target URL
        </label>
        <input
          id="target-url"
          type="text"
          value={targetUrl}
          onChange={(e) => setTargetUrl(e.target.value)}
          placeholder="https://api.example.com"
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm font-mono text-gray-900 dark:text-gray-100"
        />
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
          Must be publicly reachable. Internal IPs are rejected by the SSRF guard.
        </p>
      </div>
      <div>
        <label
          htmlFor="spec"
          className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          OpenAPI spec
        </label>
        <textarea
          id="spec"
          value={spec}
          onChange={(e) => setSpec(e.target.value)}
          placeholder="openapi: 3.0.0&#10;info:&#10;  title: ..."
          rows={10}
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm font-mono text-gray-900 dark:text-gray-100"
        />
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
          Paste the YAML or JSON document. Format is sniffed from the first
          non-whitespace byte.
        </p>
      </div>
      <div className="flex justify-end">
        <button
          type="button"
          onClick={onNext}
          disabled={!canContinue}
          className="px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
        >
          Next
          <ChevronRight className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
};

interface DataStepProps {
  dataFile: File | null;
  setDataFile: (f: File | null) => void;
  isUploading: boolean;
  error: string | null;
  onBack: () => void;
  onUpload: () => void;
}

const DataStep: React.FC<DataStepProps> = ({
  dataFile,
  setDataFile,
  isUploading,
  error,
  onBack,
  onUpload,
}) => {
  return (
    <div className="space-y-4 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
      <div>
        <label
          htmlFor="data-file"
          className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          Test vectors file
        </label>
        <input
          id="data-file"
          type="file"
          accept=".csv,.json,.yaml,.yml"
          onChange={(e) => {
            const file = e.target.files?.[0] ?? null;
            setDataFile(file);
          }}
          className="block w-full text-sm text-gray-700 dark:text-gray-300 file:mr-3 file:py-2 file:px-3 file:rounded-md file:border-0 file:text-sm file:font-medium file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100 dark:file:bg-blue-900/30 dark:file:text-blue-300"
        />
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
          CSV or JSON. Uploads directly to Tigris — the registry server never
          sees the file bytes. Cap is 64 MB at runtime.
        </p>
      </div>
      {dataFile && (
        <div className="bg-gray-50 dark:bg-gray-800 rounded-md px-3 py-2 text-sm">
          <span className="font-mono">{dataFile.name}</span>{' '}
          <span className="text-gray-500 dark:text-gray-400">
            · {formatSize(dataFile.size)}
          </span>
        </div>
      )}
      {error && (
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-800 dark:text-red-300 text-sm px-3 py-2 rounded flex items-start gap-2">
          <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
          <div>{error}</div>
        </div>
      )}
      <div className="flex justify-between">
        <button
          type="button"
          onClick={onBack}
          className="px-3 py-2 text-sm border border-gray-200 dark:border-gray-700 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800 flex items-center gap-2"
        >
          <ChevronLeft className="w-4 h-4" />
          Back
        </button>
        <button
          type="button"
          onClick={onUpload}
          disabled={!dataFile || isUploading}
          className="px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
        >
          <Upload className="w-4 h-4" />
          {isUploading ? 'Uploading…' : 'Upload to Tigris'}
        </button>
      </div>
    </div>
  );
};

interface ConfigStepProps {
  name: string;
  setName: (s: string) => void;
  vus: number;
  setVus: (n: number) => void;
  duration: string;
  setDuration: (s: string) => void;
  scenario: string;
  setScenario: (s: string) => void;
  distribution: string;
  setDistribution: (s: string) => void;
  mappings: string;
  setMappings: (s: string) => void;
  isCreating: boolean;
  error: string | null;
  uploadResult: DataDrivenUploadUrlResponse;
  onBack: () => void;
  onCreate: () => void;
}

const ConfigStep: React.FC<ConfigStepProps> = ({
  name,
  setName,
  vus,
  setVus,
  duration,
  setDuration,
  scenario,
  setScenario,
  distribution,
  setDistribution,
  mappings,
  setMappings,
  isCreating,
  error,
  uploadResult,
  onBack,
  onCreate,
}) => {
  return (
    <div className="space-y-4 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
      <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 text-green-800 dark:text-green-300 text-xs px-3 py-2 rounded">
        Test data uploaded — object key{' '}
        <code className="font-mono">{uploadResult.object_key}</code>. The
        suite's <code className="font-mono">data_url</code> expires in{' '}
        {Math.floor(uploadResult.data_expires_in_seconds / 3600)} hours.
      </div>
      <div>
        <label
          htmlFor="suite-name"
          className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          Suite name
        </label>
        <input
          id="suite-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="user-signup-load"
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100"
        />
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label
            htmlFor="vus"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            Virtual users (VUs)
          </label>
          <input
            id="vus"
            type="number"
            min={1}
            max={1000}
            value={vus}
            onChange={(e) => setVus(Number(e.target.value))}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100"
          />
        </div>
        <div>
          <label
            htmlFor="duration"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            Duration
          </label>
          <input
            id="duration"
            type="text"
            value={duration}
            onChange={(e) => setDuration(e.target.value)}
            placeholder="30s, 5m, 1h"
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm font-mono text-gray-900 dark:text-gray-100"
          />
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label
            htmlFor="scenario"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            Scenario
          </label>
          <select
            id="scenario"
            value={scenario}
            onChange={(e) => setScenario(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100"
          >
            <option value="constant">constant</option>
            <option value="ramp-up">ramp-up</option>
            <option value="spike">spike</option>
            <option value="stress">stress</option>
          </select>
        </div>
        <div>
          <label
            htmlFor="distribution"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            Data distribution
          </label>
          <select
            id="distribution"
            value={distribution}
            onChange={(e) => setDistribution(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100"
          >
            <option value="unique-per-vu">unique-per-vu</option>
            <option value="unique-per-iteration">unique-per-iteration</option>
            <option value="random">random</option>
            <option value="sequential">sequential</option>
          </select>
        </div>
      </div>
      <div>
        <label
          htmlFor="mappings"
          className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
        >
          Column mappings (optional)
        </label>
        <input
          id="mappings"
          type="text"
          value={mappings}
          onChange={(e) => setMappings(e.target.value)}
          placeholder="user_id:path.id,name:body.name"
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm font-mono text-gray-900 dark:text-gray-100"
        />
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
          Comma-separated <code className="font-mono">column:target</code> pairs.
          Targets can be <code className="font-mono">path.X</code>,{' '}
          <code className="font-mono">body.X</code>, or{' '}
          <code className="font-mono">header.X</code>.
        </p>
      </div>
      {error && (
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-800 dark:text-red-300 text-sm px-3 py-2 rounded">
          {error}
        </div>
      )}
      <div className="flex justify-between">
        <button
          type="button"
          onClick={onBack}
          className="px-3 py-2 text-sm border border-gray-200 dark:border-gray-700 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800 flex items-center gap-2"
        >
          <ChevronLeft className="w-4 h-4" />
          Back
        </button>
        <button
          type="button"
          onClick={onCreate}
          disabled={!name.trim() || isCreating}
          className="px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isCreating ? 'Creating…' : 'Create suite'}
        </button>
      </div>
    </div>
  );
};

interface CreatedStepProps {
  suite: TestSuite;
  onTrigger: () => void;
  triggerInFlight: boolean;
  triggerError: string | null;
  triggeredRun: { id: string; status: string } | null;
}

const CreatedStep: React.FC<CreatedStepProps> = ({
  suite,
  onTrigger,
  triggerInFlight,
  triggerError,
  triggeredRun,
}) => {
  return (
    <div className="space-y-4 border border-green-200 dark:border-green-800 bg-green-50 dark:bg-green-900/20 rounded-lg p-6">
      <div className="flex items-center gap-2 text-green-800 dark:text-green-300">
        <CheckCircle2 className="w-5 h-5" />
        <h2 className="text-lg font-semibold">Suite created</h2>
      </div>
      <dl className="text-sm">
        <div className="grid grid-cols-3 gap-2 py-1">
          <dt className="text-gray-600 dark:text-gray-400">Name</dt>
          <dd className="col-span-2 font-mono text-gray-900 dark:text-gray-100">
            {suite.name}
          </dd>
        </div>
        <div className="grid grid-cols-3 gap-2 py-1">
          <dt className="text-gray-600 dark:text-gray-400">Suite ID</dt>
          <dd className="col-span-2 font-mono text-xs text-gray-900 dark:text-gray-100">
            {suite.id}
          </dd>
        </div>
        <div className="grid grid-cols-3 gap-2 py-1">
          <dt className="text-gray-600 dark:text-gray-400">Kind</dt>
          <dd className="col-span-2 font-mono text-gray-900 dark:text-gray-100">
            {suite.kind}
          </dd>
        </div>
      </dl>
      {triggerError && (
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-800 dark:text-red-300 text-sm px-3 py-2 rounded">
          {triggerError}
        </div>
      )}
      {triggeredRun && (
        <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-300 text-sm px-3 py-2 rounded">
          Run queued — id <code className="font-mono">{triggeredRun.id}</code>{' '}
          (status {triggeredRun.status}). Watch progress on the{' '}
          <a className="underline" href="/cloud-test-runs">
            Cloud Test Runs
          </a>{' '}
          page.
        </div>
      )}
      <div className="flex gap-2">
        <button
          type="button"
          onClick={onTrigger}
          disabled={triggerInFlight || triggeredRun !== null}
          className="px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
        >
          <Play className="w-4 h-4" />
          {triggerInFlight ? 'Triggering…' : 'Trigger run'}
        </button>
      </div>
    </div>
  );
};

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
