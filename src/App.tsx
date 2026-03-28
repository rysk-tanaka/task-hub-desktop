// App.tsx

import { ask, open } from "@tauri-apps/plugin-dialog";
import { open as shellOpen } from "@tauri-apps/plugin-shell";
import { useEffect, useState } from "react";
import { Header } from "./components/Header";
import { Sidebar } from "./components/Sidebar";
import { SummaryView } from "./components/SummaryView";
import { WeeklySection } from "./components/WeeklySection";
import { useVault } from "./hooks/useVault";
import type { NoteKind, ViewId } from "./types";
import "./styles.css";

export default function App() {
	const {
		vaultRoot,
		summary,
		weeklyTasks,
		weekOffset,
		setWeekOffset,
		loading,
		error,
		setError,
		setVaultRoot,
		createNote,
		getAiAvailability,
		generateWeeklySummary,
	} = useVault();

	const [aiAvailable, setAiAvailable] = useState(false);

	const [activeView, setActiveView] = useState<ViewId>("summary");

	useEffect(() => {
		if (vaultRoot) {
			getAiAvailability().then(setAiAvailable);
		}
	}, [vaultRoot, getAiAvailability]);

	async function handleSelectVault() {
		try {
			const selected = await open({ directory: true, multiple: false });
			if (typeof selected === "string") {
				await setVaultRoot(selected);
			}
		} catch (e) {
			setError(String(e));
		}
	}

	async function handleCreateNote(kind: NoteKind) {
		try {
			const res = await createNote(kind);
			const label = kind === "daily" ? "Daily Note" : "Weekly Note";
			const status = res.created ? "を作成しました" : "は既に存在します";

			// Weekly Note + AI 利用可能時はサマリ生成を提案
			if (kind === "weekly" && aiAvailable) {
				const generateAi = await ask(
					`${label}${status}。\nAI 週次サマリを生成しますか？`,
					{
						title: label,
						kind: "info",
						okLabel: "生成する",
						cancelLabel: "スキップ",
					},
				);
				if (generateAi) {
					const weekMatch = res.path.match(/(\d{4}-W\d{2})\.md$/);
					if (weekMatch) {
						try {
							await generateWeeklySummary(weekMatch[1]);
						} catch (e) {
							setError(`AI サマリ生成エラー: ${String(e)}`);
						}
					}
				}
			}

			const confirmed = await ask(`${res.path}\n\nObsidian で開きますか？`, {
				title: label,
				kind: "info",
				okLabel: "開く",
				cancelLabel: "閉じる",
			});
			if (confirmed) {
				try {
					const url = `obsidian://open?path=${encodeURIComponent(res.path)}`;
					await shellOpen(url);
				} catch (e) {
					setError(String(e));
				}
			}
		} catch (e) {
			setError(String(e));
		}
	}

	// Vault 未設定時
	if (!vaultRoot) {
		return (
			<div style={styles.centered}>
				<h2 style={{ color: "var(--text)", marginBottom: 8 }}>
					Vault を選択してください
				</h2>
				<p
					style={{
						color: "var(--text-muted)",
						marginBottom: 24,
						fontSize: "var(--font-base)",
					}}
				>
					obsidian-task-hub のフォルダを指定します
				</p>
				<button
					type="button"
					style={styles.primaryButton}
					onClick={handleSelectVault}
				>
					フォルダを選択
				</button>
			</div>
		);
	}

	return (
		<div style={{ height: "100vh", display: "flex" }}>
			<Sidebar activeView={activeView} onNavigate={setActiveView} />
			<div
				style={{
					flex: 1,
					display: "flex",
					flexDirection: "column",
					background: "var(--bg)",
					color: "var(--text)",
					fontFamily: "var(--font)",
					overflow: "hidden",
				}}
			>
				<Header
					vaultRoot={vaultRoot}
					onSelectVault={handleSelectVault}
					onCreateNote={handleCreateNote}
				/>

				{error && <div style={styles.errorBanner}>{error}</div>}

				{loading && !summary && (
					<div style={styles.centered}>
						<span style={{ color: "var(--text-muted)" }}>読み込み中…</span>
					</div>
				)}

				<main style={styles.main}>
					{activeView === "summary" && summary && (
						<SummaryView summary={summary} />
					)}
					{activeView === "weekly" &&
						(weeklyTasks ? (
							<WeeklySection
								weeklyTasks={weeklyTasks}
								weekOffset={weekOffset}
								onChangeWeek={setWeekOffset}
							/>
						) : (
							<div style={styles.centered}>
								<span style={{ color: "var(--text-muted)" }}>読み込み中…</span>
							</div>
						))}
				</main>
			</div>
		</div>
	);
}

const styles: Record<
	string,
	React.CSSProperties & { WebkitAppRegion?: "drag" | "no-drag" }
> = {
	main: {
		flex: 1,
		overflow: "auto",
		padding: "20px 24px",
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
		fontSize: "var(--font-sm)",
		fontWeight: 500,
		cursor: "pointer",
	},
	errorBanner: {
		background: "#3d1a1a",
		color: "#ff6b6b",
		padding: "8px 24px",
		fontSize: "var(--font-sm)",
	},
};
