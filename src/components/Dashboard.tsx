// components/Dashboard.tsx
import { open } from "@tauri-apps/plugin-dialog";
import { useVault } from "../hooks/useVault";
import type { ProjectProgress, Task } from "../types";

// ---- サブコンポーネント ----

function StatCard({
  label,
  value,
  accent = false,
}: {
  label: string;
  value: number;
  accent?: boolean;
}) {
  return (
    <div
      style={{
        background: accent ? "var(--accent-bg)" : "var(--card-bg)",
        border: `1px solid ${accent ? "var(--accent)" : "var(--border)"}`,
        borderRadius: 8,
        padding: "16px 20px",
        minWidth: 120,
      }}
    >
      <div style={{ fontSize: 28, fontWeight: 700, color: accent ? "var(--accent)" : "var(--text)" }}>
        {value}
      </div>
      <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 2 }}>{label}</div>
    </div>
  );
}

function TaskRow({ task }: { task: Task }) {
  const isOverdue =
    task.due && new Date(task.due) < new Date(new Date().toDateString());
  return (
    <div
      style={{
        display: "flex",
        alignItems: "baseline",
        gap: 10,
        padding: "6px 0",
        borderBottom: "1px solid var(--border)",
        fontSize: 13,
      }}
    >
      <span style={{ color: isOverdue ? "var(--danger)" : "var(--text-muted)", minWidth: 86 }}>
        {task.due ?? "—"}
      </span>
      <span style={{ color: "var(--text)", flex: 1 }}>{task.text}</span>
      <span
        style={{
          fontSize: 11,
          color: "var(--text-muted)",
          maxWidth: 160,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {task.source_file}
      </span>
    </div>
  );
}

function ProgressBar({ percent }: { percent: number }) {
  return (
    <div
      style={{
        height: 4,
        background: "var(--border)",
        borderRadius: 2,
        overflow: "hidden",
        flex: 1,
      }}
    >
      <div
        style={{
          height: "100%",
          width: `${percent}%`,
          background: percent === 100 ? "var(--success)" : "var(--accent)",
          borderRadius: 2,
          transition: "width 0.3s ease",
        }}
      />
    </div>
  );
}

function ProjectRow({ project }: { project: ProjectProgress }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "7px 0",
        borderBottom: "1px solid var(--border)",
        fontSize: 13,
      }}
    >
      <span style={{ flex: 1, color: "var(--text)" }}>{project.name}</span>
      <ProgressBar percent={project.percent} />
      <span style={{ color: "var(--text-muted)", minWidth: 60, textAlign: "right" }}>
        {project.completed}/{project.total}
      </span>
    </div>
  );
}

// ---- メインダッシュボード ----

export function Dashboard() {
  const { vaultRoot, summary, loading, error, setVaultRoot, createNote } = useVault();

  async function handleSelectVault() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      await setVaultRoot(selected);
    }
  }

  async function handleCreateNote(kind: "daily" | "weekly") {
    try {
      const res = await createNote(kind);
      // Obsidianで開く（obsidian://open URLスキーム）
      const url = `obsidian://open?path=${encodeURIComponent(res.path)}`;
      window.open(url, "_blank");
    } catch (e) {
      console.error(e);
    }
  }

  // Vaultが未設定
  if (!vaultRoot) {
    return (
      <div style={styles.centered}>
        <h2 style={{ color: "var(--text)", marginBottom: 8 }}>Vault を選択してください</h2>
        <p style={{ color: "var(--text-muted)", marginBottom: 24, fontSize: 13 }}>
          obsidian-task-hub のフォルダを指定します
        </p>
        <button style={styles.primaryButton} onClick={handleSelectVault}>
          フォルダを選択
        </button>
      </div>
    );
  }

  return (
    <div style={styles.root}>
      {/* ヘッダー */}
      <header style={styles.header}>
        <div>
          <h1 style={styles.title}>Vault Companion</h1>
          <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 2 }}>
            {vaultRoot}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <button style={styles.secondaryButton} onClick={handleSelectVault}>
            Vault変更
          </button>
          <button style={styles.primaryButton} onClick={() => handleCreateNote("daily")}>
            Today
          </button>
          <button style={styles.primaryButton} onClick={() => handleCreateNote("weekly")}>
            Weekly
          </button>
        </div>
      </header>

      {error && (
        <div style={styles.errorBanner}>{error}</div>
      )}

      {loading && !summary && (
        <div style={styles.centered}>
          <span style={{ color: "var(--text-muted)" }}>読み込み中…</span>
        </div>
      )}

      {summary && (
        <main style={styles.main}>
          {/* サマリーカード */}
          <section style={styles.section}>
            <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
              <StatCard label="Inbox" value={summary.inbox_count} accent={summary.inbox_count > 0} />
              <StatCard label="今日の期限" value={summary.due_today.length} accent={summary.due_today.length > 0} />
              <StatCard label="期限超過" value={summary.overdue.length} accent={summary.overdue.length > 0} />
              <StatCard label="プロジェクト" value={summary.projects.length} />
            </div>
          </section>

          <div style={styles.columns}>
            {/* 左カラム: タスク */}
            <div style={{ flex: 1, minWidth: 0 }}>
              {summary.overdue.length > 0 && (
                <section style={styles.section}>
                  <h3 style={styles.sectionTitle}>🔴 期限超過</h3>
                  {summary.overdue.map((t, i) => (
                    <TaskRow key={i} task={t} />
                  ))}
                </section>
              )}

              {summary.due_today.length > 0 && (
                <section style={styles.section}>
                  <h3 style={styles.sectionTitle}>📅 今日の期限</h3>
                  {summary.due_today.map((t, i) => (
                    <TaskRow key={i} task={t} />
                  ))}
                </section>
              )}

              {summary.due_today.length === 0 && summary.overdue.length === 0 && (
                <section style={styles.section}>
                  <div style={{ color: "var(--text-muted)", fontSize: 13, padding: "12px 0" }}>
                    期限タスクはありません ✅
                  </div>
                </section>
              )}
            </div>

            {/* 右カラム: プロジェクト進捗 */}
            <div style={{ width: 320, flexShrink: 0 }}>
              <section style={styles.section}>
                <h3 style={styles.sectionTitle}>📁 プロジェクト</h3>
                {summary.projects.length === 0 ? (
                  <div style={{ color: "var(--text-muted)", fontSize: 13, padding: "12px 0" }}>
                    進行中のプロジェクトはありません
                  </div>
                ) : (
                  summary.projects.map((p, i) => (
                    <ProjectRow key={i} project={p} />
                  ))
                )}
              </section>
            </div>
          </div>
        </main>
      )}
    </div>
  );
}

// ---- スタイル ----

const styles: Record<string, React.CSSProperties> = {
  root: {
    height: "100vh",
    display: "flex",
    flexDirection: "column",
    background: "var(--bg)",
    color: "var(--text)",
    fontFamily: "var(--font)",
    overflow: "hidden",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "14px 24px",
    borderBottom: "1px solid var(--border)",
    background: "var(--header-bg)",
    WebkitAppRegion: "drag" as any, // タイトルバードラッグ
  },
  title: {
    fontSize: 16,
    fontWeight: 600,
    margin: 0,
    color: "var(--text)",
  },
  main: {
    flex: 1,
    overflow: "auto",
    padding: "20px 24px",
  },
  columns: {
    display: "flex",
    gap: 20,
    marginTop: 16,
    alignItems: "flex-start",
  },
  section: {
    marginBottom: 20,
  },
  sectionTitle: {
    fontSize: 13,
    fontWeight: 600,
    color: "var(--text-muted)",
    textTransform: "uppercase",
    letterSpacing: "0.06em",
    marginBottom: 8,
  },
  centered: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    padding: 40,
  },
  primaryButton: {
    background: "var(--accent)",
    color: "#fff",
    border: "none",
    borderRadius: 6,
    padding: "7px 16px",
    fontSize: 13,
    fontWeight: 500,
    cursor: "pointer",
    WebkitAppRegion: "no-drag" as any,
  },
  secondaryButton: {
    background: "transparent",
    color: "var(--text-muted)",
    border: "1px solid var(--border)",
    borderRadius: 6,
    padding: "7px 14px",
    fontSize: 13,
    cursor: "pointer",
    WebkitAppRegion: "no-drag" as any,
  },
  errorBanner: {
    background: "#3d1a1a",
    color: "#ff6b6b",
    padding: "8px 24px",
    fontSize: 12,
  },
};
