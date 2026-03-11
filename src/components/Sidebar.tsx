import type { ViewId } from "../types";

const navItems: { id: ViewId; icon: string; label: string }[] = [
	{ id: "summary", icon: "\ud83d\udcca", label: "Summary" },
	{ id: "weekly", icon: "\ud83d\udcc5", label: "Weekly" },
];

export function Sidebar({
	activeView,
	onNavigate,
}: {
	activeView: ViewId;
	onNavigate: (view: ViewId) => void;
}) {
	return (
		<nav style={styles.nav}>
			{navItems.map(({ id, icon, label }) => {
				const isActive = activeView === id;
				return (
					<button
						key={id}
						type="button"
						title={label}
						aria-label={label}
						aria-current={isActive ? "page" : undefined}
						style={{
							...styles.item,
							...(isActive ? styles.itemActive : {}),
						}}
						onClick={() => onNavigate(id)}
					>
						{icon}
					</button>
				);
			})}
		</nav>
	);
}

const styles: Record<string, React.CSSProperties> = {
	nav: {
		width: 44,
		flexShrink: 0,
		background: "var(--header-bg)",
		borderRight: "1px solid var(--border)",
		display: "flex",
		flexDirection: "column",
		alignItems: "center",
		paddingTop: 12,
		gap: 4,
	},
	item: {
		width: 36,
		height: 36,
		display: "flex",
		alignItems: "center",
		justifyContent: "center",
		fontSize: "var(--font-lg)",
		background: "transparent",
		border: "none",
		borderLeft: "3px solid transparent",
		borderRadius: 6,
		cursor: "pointer",
		padding: 0,
	},
	itemActive: {
		background: "var(--accent-bg)",
		borderLeft: "3px solid var(--accent)",
	},
};
