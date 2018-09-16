from setuptools import setup
from setuptools_rust import Binding, RustExtension



setup(name="hibike_packet",
      version="0.1",
      rust_extensions=[RustExtension("hibike_packet", "Cargo.toml", binding=Binding.RustCPython)],
      zip_safe=False)
