"""Draw the three figures of the Kalki preprint.

Run from the repository root:  python research/figures/make_figures.py
Writes fig1-architecture, fig2-gate, fig3-tradeoffs as .pdf (for LaTeX)
and .png (for the HTML render) next to this file.

Drafting conventions used in every figure:
  solid border  = implemented in kalki v0.0.1
  dashed border = designed, not implemented
  red hatching  = the agent write barrier
No figure plots a Kalki measurement. Figure 3 plots one published external
result, labelled as such, and the admission regions defined in the paper.
"""

from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.patches import FancyBboxPatch, Rectangle, Polygon  # noqa: E402

OUT = Path(__file__).resolve().parent
INK = "#111827"
MUTED = "#4b5563"
GRID = "#e5e7eb"
RED = "#b91c1c"
BLUE = "#1d4ed8"
GREEN = "#15803d"
AMBER = "#b45309"
TRUSTED = "#eef2ff"
UNTRUSTED = "#fff7ed"
GATEFILL = "#f0fdf4"

plt.rcParams.update(
    {
        "font.family": "DejaVu Sans",
        "font.size": 8,
        "mathtext.fontset": "dejavusans",
        "axes.edgecolor": INK,
        "axes.labelcolor": INK,
        "xtick.color": INK,
        "ytick.color": INK,
        "pdf.fonttype": 42,
    }
)


def save(fig, name):
    for ext in ("pdf", "png"):
        fig.savefig(OUT / f"{name}.{ext}", dpi=220, bbox_inches="tight", pad_inches=0.04)
    plt.close(fig)
    print("wrote", name)


def drafting_grid(ax, w, h, step=2.0):
    for x in range(0, int(w) + 1, int(step)):
        ax.plot([x, x], [0, h], color=GRID, lw=0.4, zorder=0)
    for y in range(0, int(h) + 1, int(step)):
        ax.plot([0, w], [y, y], color=GRID, lw=0.4, zorder=0)


def box(ax, x, y, w, h, title, lines=(), built=True, fill="white", title_color=INK, fs=7.2):
    ax.add_patch(
        FancyBboxPatch(
            (x, y),
            w,
            h,
            boxstyle="round,pad=0.02,rounding_size=0.12",
            lw=1.0,
            ls="-" if built else (0, (4, 2.5)),
            ec=INK,
            fc=fill,
            zorder=3,
        )
    )
    ax.text(x + 0.15, y + h - 0.22, title, ha="left", va="top", fontsize=fs, weight="bold", color=title_color, zorder=4)
    for i, line in enumerate(lines):
        ax.text(x + 0.15, y + h - 0.62 - i * 0.36, line, ha="left", va="top", fontsize=fs - 1.3, color=MUTED, zorder=4)


def arrow(ax, x0, y0, x1, y1, label=None, color=INK, lx=None, ly=None, ha="center", style="-|>", ls="-", fs=5.8):
    ax.annotate(
        "",
        xy=(x1, y1),
        xytext=(x0, y0),
        arrowprops=dict(arrowstyle=style, lw=0.9, color=color, ls=ls, shrinkA=1, shrinkB=1),
        zorder=5,
    )
    if label:
        ax.text(
            lx if lx is not None else (x0 + x1) / 2,
            ly if ly is not None else (y0 + y1) / 2,
            label,
            fontsize=fs,
            color=color,
            ha=ha,
            va="center",
            family="DejaVu Sans Mono",
            zorder=6,
            bbox=dict(fc="white", ec="none", pad=0.6),
        )


def hatched_barrier(ax, pts):
    """A red hatched band along a polyline: the agent write barrier."""
    xs, ys = zip(*pts)
    ax.plot(xs, ys, color=RED, lw=2.2, zorder=7)
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = int(max(abs(x1 - x0), abs(y1 - y0)) / 0.32)
        for i in range(n + 1):
            t = i / max(n, 1)
            x, y = x0 + t * (x1 - x0), y0 + t * (y1 - y0)
            ax.plot([x - 0.12, x + 0.12], [y - 0.12, y + 0.12], color=RED, lw=0.9, zorder=7)


def title_block(ax, x, y, rows):
    w, rh = 9.8, 0.36
    for i, (k, v) in enumerate(rows):
        yy = y + (len(rows) - 1 - i) * rh
        ax.add_patch(Rectangle((x, yy), w, rh, fc="white", ec=INK, lw=0.7, zorder=8))
        ax.plot([x + 1.9, x + 1.9], [yy, yy + rh], color=INK, lw=0.5, zorder=9)
        ax.text(x + 0.1, yy + rh / 2, k, fontsize=5.6, va="center", color=MUTED, zorder=9)
        ax.text(x + 2.0, yy + rh / 2, v, fontsize=5.8, va="center", color=INK, weight="bold", zorder=9)


# --------------------------------------------------------------------- Figure 1
def figure1():
    W, H = 34.0, 21.0
    fig, ax = plt.subplots(figsize=(13.6, 8.4))
    ax.set_xlim(0, W)
    ax.set_ylim(0, H)
    ax.axis("off")
    drafting_grid(ax, W, H)

    # Zone frames
    ax.add_patch(Rectangle((0.3, 0.3), 21.4, 20.4, fc="none", ec=INK, lw=1.4, zorder=1))
    ax.add_patch(Rectangle((22.1, 0.3), 11.6, 20.4, fc="none", ec=INK, lw=1.4, zorder=1))
    ax.text(0.6, 20.35, "ZONE 01  RUNTIME VERIFIABLE KERNEL", fontsize=10, weight="bold", va="top")
    ax.text(22.4, 20.35, "ZONE 02  8-STAGE ADMISSION GATE", fontsize=10, weight="bold", va="top")

    # Surfaces
    box(ax, 1.0, 18.3, 20.0, 1.25, "SURFACES", ["headless CLI (kalki run, kalki doctor): built.   terminal UI, JSONL RPC, CI mode: designed"], built=True)

    # Ring 0 band
    ax.add_patch(Rectangle((1.0, 7.4), 20.0, 9.2, fc=TRUSTED, ec=BLUE, lw=1.2, zorder=2))
    ax.text(1.2, 16.45, "RING 0  TRUSTED KERNEL (Rust, runs no model-written code in process)", fontsize=7.4, weight="bold", color=BLUE, va="top", zorder=4)

    box(ax, 1.4, 12.1, 4.6, 3.7, "STATE PLANE", ["event log (WAL)", "[u32 crc32c][u64 len][json]", "64 MiB segments, fsync", "SHA-256 chain: designed"], built=True, fill="white")
    box(ax, 6.3, 12.1, 4.4, 3.7, "BLOB STORE", ["content-addressed", "SHA-256 handle", "inline cap 4 KiB", "capture cap 64 KiB"], built=True)
    box(ax, 11.0, 12.1, 4.6, 3.7, "POLICY ENGINE", ["deny-first TOML", "first match wins", "stable rule ids", "bash: argv only"], built=True)
    box(ax, 15.9, 12.1, 4.8, 3.7, "SANDBOX PLANE", ["Landlock+seccomp+netns", "Seatbelt | AppContainer+Job", "WASI checkers", "Docker via Bollard"], built=False)
    box(ax, 1.4, 7.8, 4.6, 3.8, "CHECKPOINTS", ["(log seq, git ref)", "refs/kalki/cp/<s>/<seq>", "rewind both", "recovery aid only"], built=False)
    box(ax, 6.3, 7.8, 4.4, 3.8, "CONTEXT PLANE", ["JIT compiler per call", "prefix | rules | state", "open work | handles", "roles preserved"], built=False)
    box(ax, 11.0, 7.8, 4.6, 3.8, "MODEL GATEWAY", ["openai_responses", "anthropic_messages", "openai_compat", "loopback serve: designed"], built=True)
    box(ax, 15.9, 7.8, 4.8, 3.8, "GENOME STORE", ["~/.kalki/genome/store/<hash>", "H = <rho, G, K, M>", "load iff admitted", "or trusted signature"], built=False)

    # Ring 3 band
    ax.add_patch(Rectangle((1.0, 0.8), 20.0, 5.4, fc=UNTRUSTED, ec=AMBER, lw=1.2, zorder=2))
    ax.text(9.2, 0.95, "RING 3  UNTRUSTED EXECUTORS AND PROPOSER", fontsize=7.4, weight="bold", color=AMBER, va="bottom", zorder=4)
    box(ax, 1.4, 1.6, 4.6, 3.5, "native", ["Rust loop", "bash, apply_patch", "kernel runs each call"], built=True)
    box(ax, 6.3, 1.6, 4.4, 3.5, "pi", ["child process", "JSONL RPC", "own compaction off"], built=False)
    box(ax, 11.0, 1.6, 4.6, 3.5, "other agents", ["custom base URL", "-> gateway", "process-confined"], built=False)
    box(ax, 15.9, 1.6, 4.8, 3.5, "proposer", ["reads failures", "emits candidate h'", "no read of T or gate log"], built=False)

    # Buses
    arrow(ax, 11.0, 18.3, 11.0, 16.6, "kalki-protocol events", lx=13.6, ly=17.45)
    arrow(ax, 3.7, 5.1, 3.7, 7.8, "tool call", lx=3.7, ly=5.75, color=INK)
    arrow(ax, 8.5, 5.1, 8.5, 7.8, "JSONL RPC", lx=8.5, ly=5.75)
    arrow(ax, 13.3, 5.1, 13.3, 7.8, "HTTP, loopback", lx=13.3, ly=5.75)
    arrow(ax, 6.0, 12.5, 6.3, 12.5)
    arrow(ax, 3.7, 11.6, 3.7, 12.1, style="<|-|>")
    ax.text(0.95, 11.82, "git ref hash", fontsize=5.6, family="DejaVu Sans Mono", color=MUTED, ha="left", zorder=6)

    # Write barrier between Ring 3 and protected assets
    hatched_barrier(ax, [(1.0, 6.45), (21.0, 6.45)])
    ax.text(11.0, 6.95, "AGENT WRITE BARRIER   W_agent  intersect  (policy, gate, benchmarks, logs, genome store) = empty",
            fontsize=6.0, color=RED, ha="center", weight="bold", zorder=8, bbox=dict(fc="white", ec="none", pad=0.4))

    # Proposer -> gate
    arrow(ax, 20.7, 3.2, 22.9, 3.2, "candidate h'", lx=21.8, ly=3.6, color=GREEN)
    # Gate -> store
    arrow(ax, 22.9, 9.6, 20.7, 9.6, "admitted hash", lx=21.8, ly=10.0, color=GREEN)

    # Zone 02 pipeline
    stages = [
        ("1 Scope", "diff in genome, |Delta| <= b_t"),
        ("2 Leak", "no V-string in added text"),
        ("3 Trigger", "mean dp on F > mean delta"),
        ("4 Preserve", "lost cases on P <= m"),
        ("5 NoiseGate", "s(h') >= s(h) - tau"),
        ("6 Cost", "dc <= kappa ds, or dc < 0"),
        ("7 Tripwire", "no canary, checker, ref"),
        ("8 Rollback", "parent hash in store"),
    ]
    y = 19.6
    for name, rule in stages:
        box(ax, 22.9, y - 1.5, 7.6, 1.38, name, [rule], built=False, fill=GATEFILL, fs=7.0)
        arrow(ax, 26.7, y - 1.5, 26.7, y - 1.78, style="-|>")
        ax.annotate("", xy=(31.4, y - 0.81), xytext=(30.5, y - 0.81),
                    arrowprops=dict(arrowstyle="-|>", lw=0.7, color=RED), zorder=5)
        y -= 1.78
    ax.text(31.75, 11.5, "fail: stop at first failing stage", fontsize=5.8, color=RED, rotation=90, va="center")
    ax.add_patch(Rectangle((31.4, 4.2), 0.18, 14.6, fc=RED, ec="none", alpha=0.55, zorder=4))
    box(ax, 22.9, 2.2, 3.6, 1.7, "A  ADMIT", ["store version", "+ gate record"], built=False, fill=GATEFILL, title_color=GREEN, fs=6.8)
    box(ax, 27.0, 2.2, 4.6, 1.7, "B  VARIANT ARCHIVE", ["rejected h' with the", "cases where it helped"], built=False, fill="#fef2f2", title_color=RED, fs=6.8)
    arrow(ax, 31.5, 4.2, 30.5, 3.9, color=RED)
    arrow(ax, 26.7, 4.3, 24.7, 3.9, color=GREEN)

    title_block(ax, 22.6, 0.45, [
        ("DRAWING", "Fig. 1  Kalki system architecture"),
        ("LEGEND", "solid = implemented   dashed = designed"),
        ("REV", "kalki v0.0.1   Kalaris Labs   2026-11-11"),
    ])
    save(fig, "fig1-architecture")


# --------------------------------------------------------------------- Figure 2
def figure2():
    W, H = 22.0, 30.0
    fig, ax = plt.subplots(figsize=(7.4, 10.0))
    ax.set_xlim(0, W)
    ax.set_ylim(0, H)
    ax.axis("off")
    drafting_grid(ax, W, H)

    box(ax, 1.0, 27.6, 6.8, 1.8, "FAILURE TRACE  F", ["cases that failed under h", "from the session log"], built=True, fill=UNTRUSTED)
    box(ax, 8.6, 27.6, 6.8, 1.8, "CANDIDATE  h'", ["proposer edits rho, G, K, M", "inside edit budget b_t"], built=False, fill=UNTRUSTED)
    arrow(ax, 7.8, 28.5, 8.6, 28.5)

    stages = [
        ("G1 SCOPE", r"$\Delta(h,h')\subseteq\mathcal{P}_{gen}\ \wedge\ |\Delta|\leq b_t$", r"$O(|\Delta|)$", "no model runs"),
        ("G2 LEAK", r"$\neg\exists v\in V:\ v \sqsubseteq \mathrm{add}(\Delta)$", r"$O(|\Delta|+|V|)$", "Aho-Corasick"),
        ("G3 TRIGGER", r"$\overline{\hat p(h',x)-\hat p(h,x)}_{F} > \overline{\delta(x)}_{F}$", r"$O(2k|F|\,C_{run})$", "live runs"),
        ("G4 PRESERVE", r"$|\{x\in P: \hat p(h,x)-\hat p(h',x)>\delta(x)\}|\leq m$", r"$O(2k|P|\,C_{run})$", "live runs"),
        ("G5 NOISEGATE", r"$\bar s(h')\geq \bar s(h)-\tau$", r"$O(2k|T|\,C_{run})$", "verdict only"),
        ("G6 COST", r"$\Delta s>\tau \Rightarrow \Delta c\leq\kappa\Delta s$;  else $\Delta c<0 \vee h^{\prime}\subset h$", r"$O(1)$", "uses G5 runs"),
        ("G7 TRIPWIRE", r"$E_{eval}\cap(\mathrm{Canary}\cup\mathrm{Checker}\cup\mathrm{Ref})=\emptyset$", r"$O(|E_{eval}|)$", "event scan"),
        ("G8 ROLLBACK", r"$\mathrm{H}(h)\in S\ \wedge\ S\cap W_{agent}=\emptyset$", r"$O(1)$", "store lookup"),
    ]
    y = 26.6
    arrow(ax, 12.0, 27.6, 12.0, 26.6)
    ax.add_patch(Rectangle((19.6, 4.6), 0.22, 21.4, fc=RED, ec="none", alpha=0.5, zorder=2))
    for name, formula, cost, note in stages:
        ax.add_patch(FancyBboxPatch((1.0, y - 2.3), 17.6, 2.15, boxstyle="round,pad=0.02,rounding_size=0.12",
                                    lw=1.0, ls=(0, (4, 2.5)), ec=INK, fc=GATEFILL, zorder=3))
        ax.text(1.25, y - 0.42, name, fontsize=7.6, weight="bold", va="center", zorder=4)
        ax.text(15.0, y - 0.42, cost, fontsize=7.2, va="center", ha="left", color=BLUE, zorder=4)
        ax.text(1.25, y - 1.42, formula, fontsize=8.2, va="center", zorder=4)
        ax.text(15.0, y - 1.42, note, fontsize=6.0, va="center", color=MUTED, style="italic", zorder=4)
        ax.annotate("", xy=(19.6, y - 1.2), xytext=(18.6, y - 1.2),
                    arrowprops=dict(arrowstyle="-|>", lw=0.8, color=RED), zorder=5)
        if name != "G8 ROLLBACK":
            arrow(ax, 9.8, y - 2.3, 9.8, y - 2.75)
            ax.text(10.05, y - 2.52, "pass", fontsize=5.4, color=GREEN, va="center")
        y -= 2.75
    ax.text(20.1, 18.0, "reject on first failure", fontsize=6.2, color=RED, rotation=90, va="center")

    box(ax, 1.0, 1.0, 8.4, 2.6, "OUTCOME A  ADMIT", ["version stored by H(h') in S", "admission record in gate log", "keyed by (model, executor)"], built=False, fill=GATEFILL, title_color=GREEN)
    box(ax, 11.0, 1.0, 9.6, 2.6, "OUTCOME B  VARIANT ARCHIVE", ["h' kept with failed check", "and the cases where it helped", "proposer sees check id only"], built=False, fill="#fef2f2", title_color=RED)
    arrow(ax, 9.8, y + 0.45, 5.2, 3.6, color=GREEN, label="all eight pass", lx=7.0, ly=4.3)
    arrow(ax, 19.7, 4.6, 16.0, 3.6, color=RED)
    save(fig, "fig2-gate")


# --------------------------------------------------------------------- Figure 3
def figure3():
    fig, (a, b) = plt.subplots(1, 2, figsize=(10.4, 4.0))

    # Graph A: admission regions of the Cost and NoiseGate predicates.
    tau, kappa = 1.0, 0.25  # illustrative values, stated on the plot
    xs = (-5, 5)
    a.set_xlim(*xs)
    a.set_ylim(-2.0, 2.0)
    a.axhline(0, color=MUTED, lw=0.6)
    a.axvline(0, color=MUTED, lw=0.6)
    a.add_patch(Rectangle((-5, -2), 5 - tau, 4, fc="#fee2e2", ec="none"))
    a.add_patch(Polygon([(tau, -2), (5, -2), (5, kappa * 5), (tau, kappa * tau)], fc="#dcfce7", ec="none"))
    a.add_patch(Rectangle((-tau, -2), 2 * tau, 2, fc="#dbeafe", ec="none"))
    a.plot([tau, 5], [kappa * tau, kappa * 5], color=GREEN, lw=1.2)
    a.axvline(-tau, color=RED, lw=0.9, ls="--")
    a.axvline(tau, color=MUTED, lw=0.9, ls="--")
    a.text(-4.8, -1.2, "rejected by G5\n" + r"$\Delta s < -\tau$", fontsize=7, color=RED, va="top")
    a.text(-0.9, 1.55, "rejected\nby G6", fontsize=6.6, color=MUTED)
    a.text(1.4, 1.55, "rejected by G6\n(cost above slope)", fontsize=6.6, color=MUTED)
    a.text(-0.95, -1.75, "admissible only\nif " + r"$\Delta c<0$", fontsize=6.6, color=BLUE)
    a.text(2.2, -1.3, "admissible\n" + r"$\Delta c\leq\kappa\,\Delta s$", fontsize=7, color=GREEN)
    a.text(3.0, 1.35, r"slope $\kappa$", fontsize=7, color=GREEN)

    # Published external result (Xia et al., RRSI), as a move from h = regularized to h' = unregularized.
    s_reg, c_reg = 43.6, 2.42
    s_unreg, c_unreg = 40.3, 3.80
    ds, dc = s_unreg - s_reg, c_unreg - c_reg
    a.annotate("", xy=(ds, dc), xytext=(0, 0), arrowprops=dict(arrowstyle="-|>", color=INK, lw=1.2))
    a.plot([ds], [dc], "o", color=INK, ms=4)
    a.text(ds - 1.4, dc + 0.12, "RRSI preprint [Xia et al.]\nregularized -> unregularized\n"
           + r"$\Delta s=-3.3$, $\Delta c=+1.38$M tok/trial", fontsize=6.0, color=INK, va="bottom")
    a.set_xlabel(r"$\Delta s$  held-out score change (points)")
    a.set_ylabel(r"$\Delta c$  token cost change (M tokens per trial)")
    a.set_title("A  Cost and noise regions of the admission rule", fontsize=8.5, loc="left", weight="bold")
    a.text(0.99, 0.02, r"$\tau=1.0$, $\kappa=0.25$ M/pt: illustrative, not measured", transform=a.transAxes,
           fontsize=6, ha="right", color=MUTED, style="italic")

    # Graph B: cumulative regressions on P.
    b.set_xlim(0, 100)
    b.set_ylim(0, 30)
    b.plot([0, 100], [0, 0], color=GREEN, lw=2.0)
    b.text(3, 1.2, r"gated, $m=0$: no admitted change loses a case beyond $\delta(x)$" "\n(a property of predicate G4, not a measurement)",
           fontsize=6.4, color=GREEN, va="bottom")
    b.add_patch(Rectangle((0, 7), 100, 23, fc="#f3f4f6", ec="none"))
    b.text(50, 21, "ungated, $m>0$\nNOT YET MEASURED", ha="center", fontsize=9, color=MUTED, weight="bold")
    b.text(50, 12.5, "External reference: of 383 proposals that a replay gate rejected,\n211 (55%) fixed their trigger and broke a passing case\n[Tayebati et al., preprint, not replicated]",
           ha="center", fontsize=6.3, color=INK)
    b.set_xlabel("candidates proposed (index)")
    b.set_ylabel(r"cumulative cases lost on protected set $P$")
    b.set_title("B  Cumulative regressions on P", fontsize=8.5, loc="left", weight="bold")
    for ax in (a, b):
        ax.grid(color=GRID, lw=0.5)
        ax.set_axisbelow(True)
    fig.tight_layout()
    save(fig, "fig3-tradeoffs")


if __name__ == "__main__":
    figure1()
    figure2()
    figure3()
