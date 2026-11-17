Build `stonedgem`, a new, modular browser base written primarily in Rust, Z#, and D. It will provide the foundation for OyNIx v5.4.0 and potentially PowerOyNix. Treat `stonedgem` as the browser platform—not just a UI rewrite. 
-# also since you must use Z#, make a sytaxt extention, and major upgrade call this upgraded lang Z#++, that makes Z#++ (zharp) useful in this project

## Goals and constraints

- Use **Iced** for the browser’s native UI and as the foundation for a custom, reusable browser UI framework.
- Use **Slint** for native extension interfaces, such as extension popups, options, and permission dialogs.
- Use Rust for the browser’s core services, extension runtime, and integration between components, and other langs like D, z# as well.
- Do not use Qt, QML, Chromium, or a Chromium-based embedded browser engine. Configure Slint to use a non-Qt backend.
- Use C++ only when needed for platform integration or a component that cannot reasonably be implemented in Rust. Keep it isolated behind documented interfaces.
- Keep the project modular so OyNIx can consume `stonedgem` without depending on OyNIx-specific code.

## Architecture

Design and document clear boundaries between:

1. **Browser engine** — web-content rendering, navigation, networking, storage, and other engine services.
2. **Iced browser UI** — browser chrome, toolbars, tabs, URL/search input, menus, settings, and start page.
3. **Slint extension UI** — native extension popups, options, and permission dialogs.
4. **Rust extension runtime** — extension discovery, lifecycle, permissions, messaging, and APIs.
5. **Platform integration** — Linux desktop integration and any isolated native code.
6. **OyNIx integration** — the product layer that configures and builds on `stonedgem`.

Do not assume Iced and Slint can be embedded together seamlessly. Investigate a workable integration approach, document platform limitations, and keep their boundary explicit. If necessary, show Slint extension interfaces in separate native windows or panels.

Slint is not an HTML renderer. Do not represent existing HTML/CSS/JavaScript extension pages as supported by Slint unless they are actually rendered by a compatible engine. Document how legacy extension interfaces would need to be migrated.

## Browser engine and extensions

Before choosing an engine, inspect the project requirements and evaluate viable non-Chromium options. Document the selected engine, licensing, platform support, Rust integration, and limitations. Do not use Slint to render web pages, and do not build a pretend browser engine or placeholder that is presented as functional.

Investigate support for relevant WebExtension APIs and Manifest V2 and V3, including compatibility with existing OyNIx and PowerOyNix extensions. Treat compatibility as unproven until implemented and tested. Provide a compatibility matrix that distinguishes:

- Extensions that work unchanged.
- Extensions that need migration to Slint-based native interfaces.
- Extensions or APIs that are unsupported.
- Engine capabilities that remain unavailable or incomplete.
- you may create your own engine using c++ (23 or newer), rust, TS, minimal python, vulican, Z#, D, and, a bit of rust, use must mainly use, D, rust, c++ (24 or newer), and Z#

Do not claim full MV2 or MV3 support unless tests demonstrate it.

## Custom Iced UI framework

Build a focused browser UI layer on top of Iced, not a general-purpose framework. Provide reusable components, shared theme tokens, and documented interaction patterns for:

- Browser window chrome and navigation controls.
- URL/search and command input.
- Tabs and tab previews.
- Menus, popovers, dialogs, and settings.
- Start-page tiles and browser-specific views.

Keep the framework distinct from Iced itself so OyNIx can apply product-specific features without forking the base UI architecture.

## Visual design: Tactile Liquid Expressive

Apply a consistent visual system to Iced browser chrome and Slint extension interfaces:

- Use translucent floating surfaces for toolbars, URL/search docks, tab switchers, and popovers where the platform supports them. Provide a readable opaque fallback.
- Give controls subtle tactile depth, with distinct hover, focus, pressed, disabled, and selected states.
- Use an adaptable tonal color palette, pill-shaped primary actions, rounded tab-preview cards, and expressive but readable typography. Support light and dark themes.
- Use restrained spring-like transitions for state changes, URL/search expansion, and tab switching. Respect reduced-motion settings and accessibility needs.
- Keep colors, spacing, shapes, and typography visually consistent across Iced and Slint.

## Linux support

Support native development and packaging for Arch-based distributions and Nix-based systems, including NixOS. Document dependencies, build commands, packaging, and platform-specific limitations. Prefer reproducible builds and keep distribution-specific packaging separate from core browser logic.

## Development process and deliverables

1. Inspect the existing OyNIx and PowerOyNix codebases, if available. Record current features, Qt/QML usage, engine dependencies, extension formats, and build requirements.
2. Write an architecture proposal before implementation, including the engine decision, Iced/Slint integration approach, extension strategy, and major risks.
3. Create the `stonedgem` repository structure with clear Rust crate boundaries and documented interfaces.
4. Implement a buildable foundation incrementally. Do not replace working functionality with mock implementations presented as complete features.
5. Add tests for core services, UI behavior, extension APIs, settings/data migration, and supported platform builds.
6. Document how OyNIx v5.4.0 will consume `stonedgem`, including which existing features and extensions are supported or require migration.
7. Provide developer documentation, build instructions, and a feature-compatibility matrix.

Start by reporting the repository inspection findings and proposed architecture. Clearly identify any requirements that cannot be met until a suitable non-Chromium engine or extension API implementation is selected. Never claim a feature is implemented unless the code and tests demonstrate it.
