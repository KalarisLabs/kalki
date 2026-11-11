# Research

This folder holds the Kalki preprint from Kalaris Labs.

| File | What it is |
|---|---|
| `kalki-preprint.tex` | LaTeX source. The reference version of the paper. |
| `kalki-preprint.pdf` | Rendered PDF (13 pages). |
| `references.bib` | Bibliography. Each entry's `note` records how deeply we read the source. |
| `figures/make_figures.py` | Draws Figures 1 to 3 as PDF and PNG with matplotlib. |
| `figures/kalaris-labs-logo.png` | Lab logo used on the title page. |
| `build_pdf.py` | Renders the PDF without a TeX engine (see below). |

## Building with LaTeX

```bash
cd research
pdflatex kalki-preprint
bibtex kalki-preprint
pdflatex kalki-preprint
pdflatex kalki-preprint
```

The source needs a standard TeX Live or MiKTeX install: `amsmath`, `amsthm`,
`booktabs`, `tabularx`, `natbib`, `hyperref`, `enumitem`, `microtype`,
`orcidlink`, and `stix2`. Overleaf builds it as is.

## Building without LaTeX

```bash
python research/build_pdf.py
```

The script converts the LaTeX subset this paper uses into HTML, typesets the
math with MathJax 3.2.2 from jsDelivr, loads the STIX Two Text font from
Google Fonts (so it needs network access), and prints to PDF with headless
Edge or Chrome. Pass `--browser PATH` if neither is in a standard location.
Figures are embedded as PNG.

This route exists because the machine that produced v1 had no TeX engine. Its
output differs from LaTeX in float placement: the browser cannot float figures
or split a display equation, so some pages end with white space.

## Regenerating the figures

```bash
python research/figures/make_figures.py
```

Needs Python 3 with matplotlib and numpy.

## Status of the claims

- **Implemented** components exist in kalki v0.0.1 with tests. The 5,621 line
  count and the 89 passing tests were measured on Windows 11 with rustc 1.98.1.
- **Designed** components are specified in the paper and have no code.
- The paper reports **no Kalki benchmark measurement**. Figure 3 shows the
  analysis layout and one published number from another paper.
- Numbers from other papers come from preprints that Kalaris Labs has not
  reproduced.

## Known gaps in v1

- `kalki-preprint.tex` has not been compiled with pdflatex. Its syntax is
  standard, but expect small layout fixes on the first LaTeX build.
- Three bibliography entries (Spine, Compact-Bench, and the parallel
  sub-agents paper) have unverified author lists.
