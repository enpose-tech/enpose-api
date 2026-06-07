Enpose API — Python
===================

Pythonic wrapper over the ``enpose_api`` C ABI using `cffi
<https://cffi.readthedocs.io>`_ in ABI (``dlopen``) mode: it loads the prebuilt
shared library at run time, so no C compiler or build step is needed.

It covers the full client workflow:

* :func:`~enpose_api.discover` finds Enpose devices on the local network.
* :class:`~enpose_api.PoseStream` connects to a device and yields live
  :class:`~enpose_api.MarkerPose` updates.

API reference
-------------

.. automodule:: enpose_api
   :members:
   :undoc-members:
   :show-inheritance:
