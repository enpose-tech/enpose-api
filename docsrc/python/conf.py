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
# HTML output — alabaster theme themed with the Enpose / webgui palette.
# ---------------------------------------------------------------------------
html_theme = "alabaster"
html_title = "Enpose API — Python"
html_static_path = ["_static"]
html_css_files = ["enpose.css"]

# Dark code style; alabaster's default pygments style assumes a light page.
pygments_style = "monokai"

html_theme_options = {
    # Page + body.
    "base_bg": "#1b1b1b",
    "base_text": "#ffffff",
    "body_bg": "#1b1b1b",
    "body_text": "#ffffff",
    # Links: primary cyan, hover in the secondary (orange) accent like the webgui.
    "link": "#00ccff",
    "link_hover": "#f19640",
    # Sidebar.
    "sidebar_header": "#00ccff",
    "sidebar_text": "#a9b3b6",
    "sidebar_link": "#00ccff",
    "sidebar_link_underscore": "#11343d",
    "sidebar_list": "#ffffff",
    "sidebar_hr": "#11343d",
    "sidebar_search_button": "#11343d",
    # Code / signatures sit on the elevated (side-panel) background.
    "code_bg": "#00141a",
    "code_text": "#ffffff",
    "code_hover": "#11343d",
    "code_highlight_bg": "#00141a",
    "pre_bg": "#00141a",
    "xref_bg": "#00141a",
    "xref_border": "#11343d",
    # Anchors, target highlight and viewcode highlight.
    "highlight_bg": "#00343f",
    "viewcode_target_bg": "#00343f",
    "anchor": "#11343d",
    "anchor_hover_fg": "#00ccff",
    "anchor_hover_bg": "#11343d",
    # Rules, tables, footer.
    "hr_border": "#11343d",
    "table_border": "#11343d",
    "footer_text": "#a9b3b6",
    # Neutral grays repurposed for the dark palette (admonition backgrounds,
    # borders, footnotes, …).
    "gray_1": "#a9b3b6",
    "gray_2": "#11343d",
    "gray_3": "#11343d",
}
