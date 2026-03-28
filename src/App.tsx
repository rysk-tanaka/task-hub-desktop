// App.tsx

import { ask, open } from "@tauri-apps/plugin-dialog";
import { open as shellOpen } from "@tauri-apps/plugin-shell";
import { useCallback, useEffect, useState } from "react";
import { AiSummaryDialog } from "./components/AiSummaryDialog";
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
		saveWeeklySummary,
	} = useVault();

	const [aiAvailable, setAiAvailable] = useState(false);
	const [aiPreview, setAiPreview] = useState<{
		week: string;
		summary: string | null; // null = 生成中
		notePath: string;
	} | null>(null);

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

	const openInObsidian = useCallback(async (path: string) => {
		const confirmed = await ask(`${path}\n\nObsidian で開きますか？`, {
			title: "Note",
			kind: "info",
			okLabel: "開く",
			cancelLabel: "閉じる",
		});
		if (confirmed) {
			const url = `obsidian://open?path=${encodeURIComponent(path)}`;
			await shellOpen(url);
		}
	}, []);

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
						setAiPreview({
							week: weekMatch[1],
							summary: null,
							notePath: res.path,
						});
						try {
							const summaryText = await generateWeeklySummary(weekMatch[1]);
							setAiPreview((prev) =>
								prev ? { ...prev, summary: summaryText } : null,
							);
							return;
						} catch (e) {
							setAiPreview(null);
							setError(`AI サマリ生成エラー: ${String(e)}`);
						}
					}
				}
			}

			await openInObsidian(res.path);
		} catch (e) {
			setError(String(e));
		}
	}

	const handleAiConfirm = useCallback(async () => {
		if (!aiPreview?.summary) return;
		const { notePath } = aiPreview;
		try {
			await saveWeeklySummary(aiPreview.week, aiPreview.summary);
		} catch (e) {
			setError(`AI サマリ保存エラー: ${String(e)}`);
			return; // 保存失敗時はダイアログを閉じない
		}
		setAiPreview(null);
		await openInObsidian(notePath);
	}, [aiPreview, saveWeeklySummary, setError, openInObsidian]);

	const handleAiCancel = useCallback(async () => {
		const notePath = aiPreview?.notePath;
		setAiPreview(null);
		if (notePath) {
			await openInObsidian(notePath);
		}
	}, [aiPreview, openInObsidian]);

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

			{aiPreview && (
				<AiSummaryDialog
					summary={aiPreview.summary}
					onConfirm={handleAiConfirm}
					onCancel={handleAiCancel}
				/>
			)}
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
