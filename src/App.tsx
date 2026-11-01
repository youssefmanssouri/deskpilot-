import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import { deskpilotApi, type AppStatusDto, IpcError } from "./lib/ipc";
import "./App.css";

function App() {
  const [status, setStatus] = useState<AppStatusDto | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [testAction, setTestAction] = useState("status");
  const [intentResult, setIntentResult] = useState<string | null>(null);

  useEffect(() => {
    fetchStatus();
  }, []);

  async function fetchStatus() {
    setErrorMsg(null);
    try {
      const data = await deskpilotApi.getAppStatus();
      setStatus(data);
    } catch (err) {
      const ipcErr = IpcError.fromResponse(err);
      setErrorMsg(`[${ipcErr.code}] ${ipcErr.message}`);
    }
  }

  async function handleTestIntent(e: React.FormEvent) {
    e.preventDefault();
    setIntentResult(null);
    setErrorMsg(null);

    try {
      await deskpilotApi.validateCommandIntent({
        intentId: "test-intent-1",
        actionType: testAction,
        targetId: "system-status",
      });
      setIntentResult(`Intent '${testAction}' accepted by Rust security kernel.`);
    } catch (err) {
      const ipcErr = IpcError.fromResponse(err);
      setErrorMsg(`Intent rejected: [${ipcErr.code}] ${ipcErr.message}`);
    }
  }

  return (
    <main className="container">
      <h1>DeskPilot Core Architecture</h1>

      <div className="row">
        <a href="https://vite.dev" target="_blank" rel="noreferrer">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank" rel="noreferrer">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank" rel="noreferrer">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <section className="status-card" style={{ marginTop: "1.5rem", textAlign: "left" }}>
        <h2>Runtime Status (Typed IPC)</h2>
        {status ? (
          <ul>
            <li><strong>App Name:</strong> {status.appName}</li>
            <li><strong>Version:</strong> {status.version}</li>
            <li><strong>Kernel Status:</strong> {status.status}</li>
            <li><strong>Uptime:</strong> {status.uptimeSeconds}s</li>
            <li><strong>Environment:</strong> {status.environment}</li>
          </ul>
        ) : errorMsg ? (
          <p style={{ color: "salmon" }}>{errorMsg}</p>
        ) : (
          <p>Connecting to DeskPilot Core...</p>
        )}
        <button type="button" onClick={fetchStatus}>Refresh Status</button>
      </section>

      <section style={{ marginTop: "2rem", textAlign: "left" }}>
        <h3>Domain Intent Validation Test</h3>
        <p style={{ fontSize: "0.9rem", color: "#888" }}>
          Test the Rust trust-boundary: Allowed actions (<code>inspect</code>, <code>status</code>, <code>query</code>, <code>preview</code>). Prohibited actions will be rejected by the security kernel.
        </p>
        <form onSubmit={handleTestIntent} className="row">
          <input
            value={testAction}
            onChange={(e) => setTestAction(e.currentTarget.value)}
            placeholder="Action (e.g. status, shell_exec)"
          />
          <button type="submit">Validate Intent</button>
        </form>
        {intentResult && <p style={{ color: "lightgreen" }}>{intentResult}</p>}
        {errorMsg && <p style={{ color: "salmon" }}>{errorMsg}</p>}
      </section>
    </main>
  );
}

export default App;
