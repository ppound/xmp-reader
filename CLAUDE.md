# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a integration app for Windows File Explore and Image viewer to enable them to read xmp sidecar files. The main branch is `main`.

## Git Workflow

When doing git operations (commit, push, PR, release), always confirm the target branch with the user before executing. PRs should target `main` branch.

## Build & Deploy

Releases and installers are created on GitHub via the releases workflow, not built locally. Do not look for local installer build scripts.

## Development Workflow

After implementing a feature, always do a full build and test before committing. 

## Communication Style

When planning multi-step features, present the plan and wait for user approval before starting implementation. Do not begin executing plans automatically.

## Build & Run


## Architecture

### Entry Points


### Data Model

### Config System

## Key Conventions

## Dependencies

## Git & Deployment

- github PAT token is stored in GITHUB_TOKEN environment variable
- Releases go to GitHub via `gh release create` 
- After version bumps , update version references in: `README.md`
