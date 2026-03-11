import type { NoteKind } from "../types";

export function Header({
	vaultRoot,
	onSelectVault,
	onCreateNote,
}: {
	vaultRoot: string;
	onSelectVault: () => void;
	onCreateNote: (kind: NoteKind) => void;
}) {
	return (
		<header style={styles.header}>
			<div style={{ minWidth: 0, flex: 1 }}>
				<h1 style={styles.title}>Task Hub</h1>
				<div
					style={{
						fontSize: "var(--font-xs)",
						color: "var(--text-muted)",
						marginTop: 2,
						overflow: "hidden",
						textOverflow: "ellipsis",
						whiteSpace: "nowrap",
					}}
				>
					{vaultRoot}
				</div>
			</div>
			<div style={{ display: "flex", gap: 8, alignItems: "center" }}>
				<button
					type="button"
					style={styles.secondaryButton}
					onClick={onSelectVault}
				>
					Vault変更
				</button>
				<button
					type="button"
					style={styles.primaryButton}
					onClick={() => onCreateNote("daily")}
				>
					Today
				</button>
				<button
					type="button"
					style={styles.primaryButton}
					onClick={() => onCreateNote("weekly")}
				>
					Weekly
				</button>
			</div>
		</header>
	);
}

const styles: Record<
	string,
	React.CSSProperties & { WebkitAppRegion?: "drag" | "no-drag" }
> = {
	header: {
		display: "flex",
		justifyContent: "space-between",
		alignItems: "center",
		padding: "14px 24px",
		borderBottom: "1px solid var(--border)",
		background: "var(--header-bg)",
		WebkitAppRegion: "drag",
	},
	title: {
		fontSize: "var(--font-lg)",
		fontWeight: 600,
		margin: 0,
		color: "var(--text)",
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
		WebkitAppRegion: "no-drag",
	},
	secondaryButton: {
		background: "transparent",
		color: "var(--text-muted)",
		border: "1px solid var(--border)",
		borderRadius: 6,
		padding: "7px 14px",
		fontSize: "var(--font-sm)",
		cursor: "pointer",
		WebkitAppRegion: "no-drag",
	},
};
