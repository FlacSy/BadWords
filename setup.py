from setuptools import setup, find_packages

with open("README.md", "r") as f:
    long_description = f.read()

setup(
    name="badwords",
    version="2.0.0",
    author="FlacSy",
    author_email="flacsy.x@gmail.com",
    description="This is a library for effective moderation of content.",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/FlacSy/badwords",
    packages=find_packages(),
    package_data={'badwords': ['resource/*']},
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    license="MIT",
    include_package_data=True,
    keywords=["moderation", "content filtering", "obscenity detection", "mood analysis", "image moderation"],
)
