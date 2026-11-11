"""Render kalki-preprint.tex to PDF without a TeX engine.

This converts the restricted LaTeX subset used by the preprint into HTML,
typesets the math with MathJax, and prints the page to PDF with headless
Microsoft Edge or Google Chrome. It is a fallback for machines without
pdflatex. The LaTeX source stays the reference version of the paper.

Usage:  python -I research/build_pdf.py [--browser PATH]
Output: research/kalki-preprint.pdf and research/build/kalki-preprint.html
"""

import argparse
import html
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TEX = HERE / "kalki-preprint.tex"
BIB = HERE / "references.bib"
BUILD = HERE / "build"
OUT_HTML = BUILD / "kalki-preprint.html"
OUT_PDF = HERE / "kalki-preprint.pdf"

BROWSERS = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
]

MATHJAX = "https://cdn.jsdelivr.net/npm/mathjax@3.2.2/es5/tex-svg.js"


# ---------------------------------------------------------------- helpers

def strip_comments(s):
    return re.sub(r"(?<!\\)%.*", "", s)


def group(s, i):
    """Return (content, end) of the brace group starting at s[i] == '{'."""
    assert s[i] == "{", s[i : i + 30]
    depth = 0
    for j in range(i, len(s)):
        c = s[j]
        if c == "\\":
            continue
        if c == "{" and s[j - 1] != "\\":
            depth += 1
        elif c == "}" and s[j - 1] != "\\":
            depth -= 1
            if depth == 0:
                return s[i + 1 : j], j + 1
    raise ValueError("unbalanced braces near: " + s[i : i + 60])


def optarg(s, i):
    """Return (content, end) of an optional [..] argument at s[i], or (None, i)."""
    if i < len(s) and s[i] == "[":
        j = s.index("]", i)
        return s[i + 1 : j], j + 1
    return None, i


def env_body(s, i, name):
    """s[i:] starts just after \\begin{name}. Return (body, end)."""
    open_t, close_t = "\\begin{%s}" % name, "\\end{%s}" % name
    depth, j = 1, i
    while True:
        a = s.find(open_t, j)
        b = s.find(close_t, j)
        if b < 0:
            raise ValueError("missing " + close_t)
        if 0 <= a < b:
            depth, j = depth + 1, a + len(open_t)
            continue
        depth -= 1
        if depth == 0:
            return s[i:b], b + len(close_t)
        j = b + len(close_t)


def replace_cmd(s, name, fn, nargs=1):
    """Replace \\name{a}{b}.. using fn(*args), handling nested braces."""
    pat = "\\" + name + "{"
    out, i = [], 0
    while True:
        k = s.find(pat, i)
        if k < 0:
            out.append(s[i:])
            return "".join(out)
        out.append(s[i:k])
        j = k + len(pat) - 1
        args = []
        for _ in range(nargs):
            while j < len(s) and s[j] in " \n":
                j += 1
            a, j = group(s, j)
            args.append(a)
        out.append(fn(*args))
        i = j


# ---------------------------------------------------------------- state

class Doc:
    def __init__(self, body, preamble, bib):
        self.body = body
        self.preamble = preamble
        self.bib = bib
        self.labels = {}
        self.cites = []
        self.fig = self.tab = 0
        self.sec = self.sub = 0
        self.thm = {}
        self.prepass()

    def prepass(self):
        sec = sub = fig = tab = 0
        current = None
        in_math = False
        tok = re.compile(
            r"\\(section|subsection)\{|\\begin\{(figure\*?|table\*?|equation|align)\}"
            r"|\\end\{(equation|align)\}|\\label\{([^}]*)\}|\\cite[pt]?\{([^}]*)\}"
        )
        for m in tok.finditer(self.body):
            if m.group(1) == "section":
                sec, sub = sec + 1, 0
                current = str(sec)
            elif m.group(1) == "subsection":
                sub += 1
                current = "%d.%d" % (sec, sub)
            elif m.group(2):
                env = m.group(2)
                if env.startswith("figure"):
                    fig += 1
                    current = str(fig)
                elif env.startswith("table"):
                    tab += 1
                    current = str(tab)
                else:
                    in_math = True
            elif m.group(3):
                in_math = False
            elif m.group(4) is not None:
                if not in_math:
                    self.labels[m.group(4)] = current
            elif m.group(5):
                for k in m.group(5).split(","):
                    k = k.strip()
                    if k not in self.cites:
                        self.cites.append(k)

    # ------------------------------------------------------------ inline

    def cite(self, keys):
        nums = sorted(self.cites.index(k.strip()) + 1 for k in keys.split(","))
        parts, run = [], [nums[0]]
        for n in nums[1:]:
            if n == run[-1] + 1:
                run.append(n)
            else:
                parts.append(run)
                run = [n]
        parts.append(run)
        txt = []
        for r in parts:
            if len(r) >= 3:
                txt.append('<a href="#ref%d">%d</a>&ndash;<a href="#ref%d">%d</a>' % (r[0], r[0], r[-1], r[-1]))
            else:
                txt.extend('<a href="#ref%d">%d</a>' % (n, n) for n in r)
        return "[" + ", ".join(txt) + "]"

    def inline(self, s):
        keep = []

        def protect(raw):
            keep.append(raw)
            return "\x00%d\x00" % (len(keep) - 1)

        s = s.strip()
        # math first, so nothing below touches it
        s = re.sub(r"(?<!\\)\$(.+?)(?<!\\)\$", lambda m: protect("\\(" + html.escape(m.group(1), quote=False) + "\\)"), s, flags=re.S)
        s = re.sub(r"\\eqref\{([^}]*)\}", lambda m: protect("\\eqref{" + m.group(1) + "}"), s)
        s = html.escape(s, quote=False)
        s = re.sub(r"\\cite[pt]?\{([^}]*)\}", lambda m: protect(self.cite(m.group(1))), s)
        s = re.sub(r"\\ref\{([^}]*)\}", lambda m: protect(self.labels.get(m.group(1), "??")), s)
        s = replace_cmd(s, "orcidlink", lambda i: protect(
            '<a class="orcid" href="https://orcid.org/%s" title="ORCID %s">%s</a>' % (i, i, ORCID_SVG)))
        s = s.replace("\\textbullet", "•").replace("\\,", " ")
        s = replace_cmd(s, "href",lambda u, t: protect('<a href="%s">' % u) + t + protect("</a>"), 2)
        s = replace_cmd(s, "url", lambda u: protect('<a href="%s">%s</a>' % (u, u)))
        s = replace_cmd(s, "textbf", lambda t: protect("<b>") + t + protect("</b>"))
        s = replace_cmd(s, "emph", lambda t: protect("<i>") + t + protect("</i>"))
        s = replace_cmd(s, "texttt", lambda t: protect("<code>") + t + protect("</code>"))
        s = re.sub(r"\\\\(\[[^\]]*\])?", lambda m: protect("<br>"), s)
        subs = [
            ("{,}", ","), ("\\&amp;", "&amp;"), ("\\%", "%"), ("\\_", "_"),
            ('\\"o', "ö"), ('\\"u', "ü"), ("\\quad", "\u2003"), ("\\ ", " "),
            ("\\small", ""), ("\\centering", ""), ("~", "\u00a0"), ("--", "\u2013"),
        ]
        for a, b in subs:
            s = s.replace(a, b)
        s = s.replace("{", "").replace("}", "")
        s = re.sub(r"\s+", " ", s).strip()
        while "\x00" in s:
            s = re.sub(r"\x00(\d+)\x00", lambda m: keep[int(m.group(1))], s)
        return s

    # ------------------------------------------------------------ blocks

    def table(self, spec, body, cls=""):
        cols = re.findall(r"p\{([^}]*)\}|([lcrX])", spec.replace("@{}", ""))
        widths = [w for w, _ in cols]
        body = re.sub(r"\\(toprule|bottomrule)", "", body)
        head = None
        if "\\midrule" in body:
            head, body = body.split("\\midrule", 1)
        def rows(t):
            out = []
            for r in re.split(r"\\\\", t):
                if r.strip():
                    out.append([c for c in re.split(r"(?<!\\)&", r)])
            return out
        h = ['<table class="tab %s">' % cls]
        if any(widths):
            h.append("<colgroup>" + "".join('<col style="width:%s">' % w if w else "<col>" for w in widths) + "</colgroup>")
        if head:
            h.append("<thead>")
            for r in rows(head):
                h.append("<tr>" + "".join("<th>%s</th>" % self.inline(c) for c in r) + "</tr>")
            h.append("</thead>")
        h.append("<tbody>")
        for r in rows(body):
            h.append("<tr>" + "".join("<td>%s</td>" % self.inline(c) for c in r) + "</tr>")
        h.append("</tbody></table>")
        return "".join(h)

    def float_(self, body, kind):
        cap_i = body.find("\\caption{")
        caption = ""
        if cap_i >= 0:
            caption, _ = group(body, cap_i + len("\\caption"))
        label = re.search(r"\\label\{([^}]*)\}", body)
        num = self.labels.get(label.group(1)) if label else ""
        name = "Figure" if kind == "figure" else "Table"
        inner = ""
        g = re.search(r"\\includegraphics(\[[^\]]*\])?\{([^}]*)\}", body)
        if g:
            src = "../" + re.sub(r"\.pdf$", ".png", g.group(2))
            w = re.search(r"width=([\d.]+)\\textwidth", g.group(1) or "")
            pct = float(w.group(1)) * 100 if w else 100
            inner = '<img src="%s" style="width:%.0f%%">' % (src, pct)
        t = body.find("\\begin{tabularx}")
        if t >= 0:
            j = t + len("\\begin{tabularx}")
            _, j = group(body, j)
            spec, j = group(body, j)
            tb, _ = env_body(body, j, "tabularx")
            inner = self.table(spec, tb)
        cap = '<figcaption><b>%s %s:</b> %s</figcaption>' % (name, num, self.inline(caption))
        anchor = ' id="%s"' % label.group(1) if label else ""
        return '<figure class="%s"%s>%s%s</figure>' % (kind, anchor, inner, cap)

    def blocks(self, s):
        out, para = [], []

        def flush():
            text = " ".join(para).strip()
            para.clear()
            if text:
                text = self.inline(text)
                text = re.sub(r"^(<br>\s*)+", "", text)
                if text:
                    out.append("<p>%s</p>" % text)

        starts = re.compile(
            r"\n[ \t]*\n|\\begin\{([a-z*]+)\}|\\(section|subsection|paragraph)\{|\\maketitle"
            r"|\\includegraphics(\[[^\]]*\])?\{|\\bibliographystyle\{[^}]*\}|\\bibliography\{[^}]*\}"
            r"|\\label\{[^}]*\}"
        )
        i = 0
        while True:
            m = starts.search(s, i)
            if not m:
                para.append(s[i:])
                flush()
                return "\n".join(out)
            para.append(s[i : m.start()])
            tok = m.group(0)
            i = m.end()
            if tok.strip() == "":
                flush()
            elif tok.startswith("\\label"):
                pass
            elif tok == "\\maketitle":
                flush()
                out.append(self.title())
            elif tok.startswith("\\bibliographystyle"):
                pass
            elif tok.startswith("\\bibliography"):
                flush()
                out.append(self.bibliography())
            elif tok.startswith("\\includegraphics"):
                flush()
                path, i = group(s, m.end() - 1)
                w = re.search(r"width=([\d.]+)\\textwidth", m.group(3) or "")
                pct = float(w.group(1)) * 100 if w else 100
                out.append('<div class="center"><img src="../%s" style="width:%.0f%%"></div>' % (re.sub(r"\.pdf$", ".png", path), pct))
            elif m.group(2):
                flush()
                title, i = group(s, m.end() - 1)
                kind = m.group(2)
                if kind == "paragraph":
                    para.append("\\textbf{%s}\u2003" % title)
                    continue
                lab = re.match(r"\s*\\label\{([^}]*)\}", s[i:])
                anchor = ' id="%s"' % lab.group(1) if lab else ""
                if kind == "section":
                    self.sec, self.sub = self.sec + 1, 0
                    out.append('<h2%s><span class="num">%d</span>%s</h2>' % (anchor, self.sec, self.inline(title)))
                else:
                    self.sub += 1
                    out.append('<h3%s><span class="num">%d.%d</span>%s</h3>' % (anchor, self.sec, self.sub, self.inline(title)))
            else:
                flush()
                env = m.group(1)
                body, i = env_body(s, i, env)
                out.append(self.env(env, body))

    def env(self, env, body):
        if env in ("equation", "align"):
            return '<div class="eq">\\begin{%s}%s\\end{%s}</div>' % (env, html.escape(body, quote=False), env)
        if env in ("figure", "figure*"):
            return self.float_(body, "figure")
        if env in ("table", "table*"):
            return self.float_(body, "table")
        if env == "center":
            return '<div class="center">%s</div>' % self.blocks(body)
        if env == "tabularx":
            j = 0
            _, j = group(body, j)
            spec, j = group(body, j)
            return self.table(spec, body[j:], "meta")
        if env == "abstract":
            return '<div class="abstract"><h4>Abstract</h4>%s</div>' % self.blocks(body)
        if env in ("itemize", "enumerate", "description"):
            tag = "ol" if env == "enumerate" else "ul"
            if env == "description":
                tag = "dl"
            body = re.sub(r"^\[[^\]]*\]", "", body.strip()) if env == "itemize" else body
            items = re.split(r"\\item", body)[1:]
            h = ["<%s>" % tag]
            for it in items:
                if env == "description":
                    lab, k = optarg(it, 0)
                    h.append("<dt>%s</dt><dd>%s</dd>" % (self.inline(lab or ""), self.inline(it[k:])))
                else:
                    h.append("<li>%s</li>" % self.inline(it))
            h.append("</%s>" % tag)
            return "".join(h)
        if env.startswith("tabularx"):
            pass
        if env in ("definition", "proposition", "remark"):
            name, k = optarg(body, 0)
            n = self.thm.get(env, 0) + 1
            self.thm[env] = n
            head = "<b>%s %d</b>" % (env.capitalize(), n)
            if name:
                head += " (%s)" % self.inline(name)
            inner = self.blocks(body[k:])
            inner = inner.replace("<p>", "<p>" + head + ". ", 1)
            return '<div class="thm">%s</div>' % inner
        if env == "proof":
            name, k = optarg(body, 0)
            inner = self.blocks(body[k:])
            inner = inner.replace("<p>", "<p><i>%s.</i> " % (name or "Proof"), 1)
            return '<div class="proof">%s<span class="qed">&#9633;</span></div>' % inner
        raise ValueError("unsupported environment: " + env)

    # ------------------------------------------------------------ frontmatter, bib

    def pre(self, cmd):
        k = self.preamble.find("\\" + cmd + "{")
        v, _ = group(self.preamble, k + len(cmd) + 1)
        return v

    def title(self):
        title = self.pre("newcommand{\\papertitle}") if "\\papertitle" in self.preamble else self.pre("title")
        author = self.pre("author")
        lines = [self.inline(x) for x in re.split(r"\\\\", author)]
        return (
            '<header><img class="logo" src="../figures/kalaris-labs-logo.png">'
            '<hr class="heavy"><h1>%s</h1><hr class="light">'
            '<div class="author"><div class="name">%s</div>%s</div>'
            '<div class="date">%s</div></header>'
            % (self.inline(title), lines[0], "".join("<div>%s</div>" % x for x in lines[1:]), self.inline(self.pre("date")))
        )

    def bibliography(self):
        h = ['<h2 class="refs">References</h2><ol class="bib">']
        for n, key in enumerate(self.cites, 1):
            e = self.bib.get(key)
            h.append('<li id="ref%d">%s</li>' % (n, self.inline(format_bib(e)) if e else "??"))
        h.append("</ol>")
        return "".join(h)


# ---------------------------------------------------------------- bibtex

def parse_bib(text):
    entries = {}
    for m in re.finditer(r"@(\w+)\s*\{\s*([^,\s]+)\s*,", text):
        start = m.end() - 1
        depth, j = 0, text.index("{", m.start())
        for j in range(text.index("{", m.start()), len(text)):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
        body = text[start + 1 : j]
        fields, i = {}, 0
        for fm in re.finditer(r"(\w+)\s*=\s*\{", body):
            if fm.start() < i:
                continue
            v, i = group(body, fm.end() - 1)
            fields[fm.group(1).lower()] = re.sub(r"\s+", " ", v)
        fields["_type"] = m.group(1).lower()
        entries[m.group(2)] = fields
    return entries


def authors(a):
    names = [x.strip() for x in a.split(" and ")]
    etal = names and names[-1] == "others"
    if etal:
        names = names[:-1]
    if etal:
        return ", ".join(names) + " et al."
    if len(names) <= 2:
        return " and ".join(names)
    return ", ".join(names[:-1]) + ", and " + names[-1]


def format_bib(e):
    parts = []
    def sentence(t):
        return t if t.rstrip("}").endswith(".") else t + "."

    if "author" in e:
        parts.append(sentence(authors(e["author"])))
    parts.append(sentence(e.get("title", "")))
    venue = []
    if "journal" in e:
        j = "\\emph{%s}" % e["journal"]
        if "volume" in e:
            j += ", %s" % e["volume"]
            if "number" in e:
                j += "(%s)" % e["number"]
        if "pages" in e:
            j += ":%s" % e["pages"]
        venue.append(j)
    if "howpublished" in e:
        venue.append(e["howpublished"])
    if "year" in e:
        venue.append(e["year"])
    if venue:
        parts.append(", ".join(venue) + ".")
    if "eprint" in e:
        parts.append("arXiv:%s." % e["eprint"])
    if "note" in e:
        parts.append(e["note"])
    return " ".join(parts)


# ---------------------------------------------------------------- page

ORCID_SVG = (
    '<svg viewBox="0 0 256 256" width="0.9em" height="0.9em" style="vertical-align:-0.08em">'
    '<circle cx="128" cy="128" r="128" fill="#A6CE39"/><g fill="#fff">'
    '<rect x="72" y="106" width="18" height="94"/><circle cx="81" cy="80" r="12"/>'
    '<path fill-rule="evenodd" d="M110 106h38c38 0 56 25 56 47s-18 47-56 47h-38z'
    'M128 122v62h20c29 0 38-17 38-31s-9-31-38-31z"/></g></svg>'
)

FONTS = "https://fonts.googleapis.com/css2?family=STIX+Two+Text:ital,wght@0,400..700;1,400..700&display=block"

CSS = r"""
@page { size: A4; margin: 24mm 25mm 22mm 25mm;
  @bottom-center { content: counter(page); font: 10pt "STIX Two Text", "Times New Roman", serif; color: #222; } }
html { font-family: "STIX Two Text", "Times New Roman", serif; font-weight: 460; font-size: 11pt;
  line-height: 1.5; color: #000; }
body { margin: 0; }
b, strong, th, dt { font-weight: 700; }
p { margin: 0 0 0.75em 0; text-align: justify; hyphens: auto; }
a { color: #1f3f7a; text-decoration: none; }
a[href^="http"] { color: #b8540f; }
code { font-family: "Consolas", "DejaVu Sans Mono", monospace; font-size: 0.85em; overflow-wrap: anywhere; }
header { text-align: center; margin: 0 0 8mm 0; }
header .logo { width: 26%; margin-bottom: 5mm; }
hr.heavy { border: 0; border-top: 3px solid #000; margin: 0 0 6mm 0; }
hr.light { border: 0; border-top: 0.8px solid #000; margin: 5mm 0 7mm 0; }
h1 { font-size: 19pt; line-height: 1.3; margin: 0 4mm; font-weight: 700; }
.author .name { font-size: 12.5pt; }
.author div { font-size: 11.5pt; line-height: 1.45; }
a.orcid { margin-left: 0.15em; }
.date { margin-top: 3mm; font-size: 11pt; color: #6b6b6b; }
h2 { font-size: 14.5pt; font-weight: 700; margin: 1.6em 0 0.6em 0; break-after: avoid; }
h3 { font-size: 12pt; font-weight: 700; margin: 1.3em 0 0.5em 0; break-after: avoid; }
h2 .num { display: inline-block; min-width: 1.5em; }
h3 .num { display: inline-block; min-width: 2.3em; }
h2.refs { margin-top: 1.8em; }
.center { text-align: center; margin: 0.3em 0 0.6em 0; }
.abstract { margin: 4mm 13mm 9mm 13mm; }
.abstract h4 { text-align: center; font-size: 12pt; font-weight: 700; margin: 0 0 3mm 0; }
.abstract p { text-align: justify; line-height: 1.45; }
figure { margin: 1.2em 0 1.4em 0; break-inside: avoid; text-align: center; }
figure img { max-width: 100%; }
figcaption { font-size: 9.8pt; text-align: justify; margin-top: 0.6em; line-height: 1.4; }
table.tab { border-collapse: collapse; width: 100%; font-size: 9.2pt; line-height: 1.35; text-align: left;
  border-top: 1.4px solid #000; border-bottom: 1.4px solid #000; margin-bottom: 0.3em; }
table.tab th { font-weight: 700; border-bottom: 0.8px solid #000; padding: 3px 5px; vertical-align: bottom; }
table.tab td { padding: 3px 5px; vertical-align: top; }
table.meta { width: 92%; margin: 0 auto 0.4em auto; font-size: 8.8pt; }
table.meta td:first-child { font-weight: 700; white-space: nowrap; width: 8.5em; }
.center table.meta td { text-align: left; }
.eq { margin: 0.5em 0; break-inside: avoid; }
.thm { margin: 0.6em 0; }
.thm p:first-child { font-style: italic; }
.proof { margin: 0.4em 0 0.8em 0; }
.proof p:last-of-type { display: inline; }
.qed { float: right; }
ul, ol { margin: 0.2em 0 0.7em 0; padding-left: 1.5em; }
li { margin-bottom: 0.25em; text-align: justify; }
dl { margin: 0.3em 0 0.8em 0.8em; }
dt { font-weight: 700; margin-top: 0.3em; }
dd { margin: 0 0 0.2em 1.4em; text-align: justify; }
ol.bib { font-size: 8.8pt; padding-left: 2.2em; }
ol.bib li { margin-bottom: 0.35em; text-align: left; }
mjx-container[display="true"] { margin: 0.4em 0 !important; }
"""

MJ_CONFIG = r"""
window.MathJax = {
  tex: { tags: 'ams', inlineMath: [['\\(', '\\)']], displayMath: [['\\[', '\\]']],
         processEnvironments: true, processRefs: true },
  svg: { fontCache: 'global', scale: 0.96 },
  startup: { pageReady: () => MathJax.startup.defaultPageReady().then(() => { document.body.dataset.mj = 'done'; }) }
};
"""


def build_html():
    src = strip_comments(TEX.read_text(encoding="utf-8"))
    pre, rest = src.split("\\begin{document}", 1)
    body = rest.split("\\end{document}", 1)[0]
    doc = Doc(body, pre, parse_bib(BIB.read_text(encoding="utf-8")))
    content = doc.blocks(body)
    missing = [k for k in doc.cites if k not in doc.bib]
    if missing:
        sys.exit("citation keys missing from references.bib: " + ", ".join(missing))
    if "??" in content:
        sys.exit("unresolved reference (??) in output")
    page = (
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">"
        "<title>Kalki preprint</title><link rel=\"stylesheet\" href=\"%s\"><style>%s</style><script>%s</script>"
        "<script async src=\"%s\"></script></head><body>%s</body></html>"
        % (FONTS, CSS, MJ_CONFIG, MATHJAX, content)
    )
    BUILD.mkdir(exist_ok=True)
    OUT_HTML.write_text(page, encoding="utf-8")
    return len(doc.cites)


def print_pdf(browser):
    tmp_profile = BUILD / "profile"
    cmd = [
        browser, "--headless=new", "--disable-gpu", "--no-first-run",
        "--user-data-dir=" + str(tmp_profile), "--no-pdf-header-footer",
        "--virtual-time-budget=45000", "--run-all-compositor-stages-before-draw",
        "--print-to-pdf=" + str(OUT_PDF), OUT_HTML.resolve().as_uri(),
    ]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=180)
    shutil.rmtree(tmp_profile, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--browser")
    ap.add_argument("--html-only", action="store_true")
    a = ap.parse_args()
    n = build_html()
    print("wrote %s (%d references)" % (OUT_HTML, n))
    if a.html_only:
        return
    browser = a.browser or next((b for b in BROWSERS if os.path.exists(b)), None)
    if not browser:
        sys.exit("no Chromium-based browser found; pass --browser PATH")
    print_pdf(browser)
    print("wrote %s" % OUT_PDF)


if __name__ == "__main__":
    main()
