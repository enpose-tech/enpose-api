# Sphinx configuration for the Enpose API Python binding.
#
# Built by the top-level CMake `docs` target into docs/python/. The cffi
# package is mocked so autodoc imports `enpose_api` without dlopening the
# shared library at doc-build time.

import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
# The cffi binding package lives at <repo-root>/python.
sys.path.insert(0, os.path.abspath(os.path.join(_here, "..", "..", "python")))

project = "Enpose API — Python"
author = "Enpose"
release = "0.1.0"

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.napoleon",
    "sphinx.ext.viewcode",
    "sphinx.ext.intersphinx",
]

# Import enpose_api without its native dependency: cffi (and thus the dlopen of
# the shared library at module import) is replaced by a stub.
autodoc_mock_imports = ["cffi"]
autodoc_member_order = "bysource"
autodoc_typehints = "description"
autodoc_default_options = {
    "members": True,
    "undoc-members": False,
    "show-inheritance": True,
}
napoleon_google_docstring = True
napoleon_numpy_docstring = False

intersphinx_mapping = {"python": ("https://docs.python.org/3", None)}

# ---------------------------------------------------------------------------
# HTML output — furo theme themed with the Enpose / webgui palette.
# Palette mirrors webgui/src/my_ui.rs (PRIMARY_COLOR #00ccff,
# PRIMARY_COLOR_DARK #007f9f, SECONDARY_COLOR #f19640, side panel #00141a).
# ---------------------------------------------------------------------------
html_theme = "furo"
html_title = "Enpose API — Python"
html_static_path = ["_static"]
html_css_files = ["enpose.css"]

_enpose_palette = {
    "color-brand-primary": "#00ccff",
    "color-brand-content": "#00ccff",
    "color-brand-visited": "#00ccff",
    "color-background-primary": "#1b1b1b",
    "color-background-secondary": "#00141a",
    "color-background-hover": "#11343d",
    "color-background-border": "#11343d",
    "color-foreground-primary": "#ffffff",
    "color-foreground-secondary": "#a9b3b6",
    "color-foreground-muted": "#a9b3b6",
    "color-foreground-border": "#11343d",
    "color-api-name": "#00ccff",
    "color-api-pre-name": "#f19640",
    "color-api-keyword": "#a9b3b6",
    "color-highlight-on-target": "#00343f",
    "color-inline-code-background": "#00141a",
    "color-code-background": "#00141a",
    "color-code-foreground": "#ffffff",
}

# Use the same (dark) palette for both colour schemes so the docs always render
# in the webgui's dark look, regardless of the visitor's OS preference.
html_theme_options = {
    "light_css_variables": _enpose_palette,
    "dark_css_variables": _enpose_palette,
}
