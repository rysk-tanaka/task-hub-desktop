import type { CSSProperties } from "react";

/** AI 週次サマリのプレビュー・確認ダイアログ */
export function AiSummaryDialog({
	summary,
	onConfirm,
	onCancel,
}: {
	summary: string | null; // null = 生成中
	onConfirm: () => void;
	onCancel: () => void;
}) {
	const loading = summary === null;

	return (
		<div style={styles.overlay}>
			<div style={styles.dialog}>
				<h2 style={styles.heading}>AI 週次サマリ</h2>

				{loading ? (
					<div style={styles.loadingContainer}>
						<div className="ai-spinner" />
						<span style={styles.loadingText}>生成中…</span>
					</div>
				) : (
					<div
						style={styles.content}
						className="ai-summary-content"
						// biome-ignore lint/security/noDangerouslySetInnerHtml: markdown preview from local AI
						dangerouslySetInnerHTML={{ __html: markdownToHtml(summary) }}
					/>
				)}

				<div style={styles.actions}>
					<button
						type="button"
						style={styles.cancelButton}
						onClick={onCancel}
						disabled={loading}
					>
						破棄
					</button>
					<button
						type="button"
						style={{
							...styles.confirmButton,
							...(loading ? styles.disabledButton : {}),
						}}
						onClick={onConfirm}
						disabled={loading}
					>
						追記する
					</button>
				</div>
			</div>
		</div>
	);
}

/** Markdown のサブセットを HTML に変換する（太字・箇条書き・段落） */
function markdownToHtml(md: string): string {
	const lines = md.split("\n");
	const parts: string[] = [];
	let inList = false;

	for (const line of lines) {
		const trimmed = line.trim();

		if (trimmed.startsWith("- ")) {
			if (!inList) {
				parts.push("<ul>");
				inList = true;
			}
			parts.push(`<li>${inlineFormat(trimmed.slice(2))}</li>`);
		} else {
			if (inList) {
				parts.push("</ul>");
				inList = false;
			}
			if (trimmed !== "") {
				parts.push(`<p>${inlineFormat(trimmed)}</p>`);
			}
		}
	}

	if (inList) {
		parts.push("</ul>");
	}

	return parts.join("");
}

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;")
		.replace(/"/g, "&quot;")
		.replace(/'/g, "&#039;");
}

function inlineFormat(text: string): string {
	const escaped = escapeHtml(text);
	return escaped.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
}

const styles: Record<string, CSSProperties> = {
	overlay: {
		position: "fixed",
		inset: 0,
		background: "rgba(0, 0, 0, 0.5)",
		display: "flex",
		alignItems: "center",
		justifyContent: "center",
		zIndex: 100,
	},
	dialog: {
		maxWidth: 560,
		width: "90vw",
		maxHeight: "70vh",
		display: "flex",
		flexDirection: "column",
		background: "var(--card-bg)",
		color: "var(--text)",
		border: "1px solid var(--border)",
		borderRadius: 12,
		padding: 0,
		overflow: "hidden",
	},
	heading: {
		fontSize: "var(--font-base)",
		fontWeight: 600,
		margin: 0,
		padding: "16px 20px 12px",
		borderBottom: "1px solid var(--border)",
	},
	content: {
		flex: 1,
		overflow: "auto",
		padding: "16px 20px",
		fontSize: "var(--font-sm)",
		lineHeight: 1.7,
	},
	loadingContainer: {
		flex: 1,
		display: "flex",
		flexDirection: "column",
		alignItems: "center",
		justifyContent: "center",
		padding: "48px 20px",
		gap: 16,
	},
	loadingText: {
		color: "var(--text-muted)",
		fontSize: "var(--font-sm)",
	},
	actions: {
		display: "flex",
		justifyContent: "flex-end",
		gap: 8,
		padding: "12px 20px",
		borderTop: "1px solid var(--border)",
	},
	cancelButton: {
		background: "transparent",
		color: "var(--text-muted)",
		border: "1px solid var(--border)",
		borderRadius: 6,
		padding: "7px 16px",
		fontSize: "var(--font-sm)",
		cursor: "pointer",
	},
	confirmButton: {
		background: "var(--accent)",
		color: "#fff",
		border: "none",
		borderRadius: 6,
		padding: "7px 16px",
		fontSize: "var(--font-sm)",
		fontWeight: 500,
		cursor: "pointer",
	},
	disabledButton: {
		opacity: 0.4,
		cursor: "not-allowed",
	},
};
