import { describe, expect, it } from "vitest";

import { canAdvance, canApply, capabilityFor, initialState, reduce } from "../src/lib/wizard";
import { catalog, detection, preview, run } from "./fixtures";

describe("platform capability gating", () => {
  it("allows nothing before this Mac has been checked", () => {
    const capability = capabilityFor(null);
    expect(capability.canApply).toBe(false);
    expect(capability.canPreview).toBe(false);
    expect(capability.reason).toContain("has not checked this computer yet");
  });

  it("allows preview and apply on macOS with Brave installed", () => {
    expect(capabilityFor(detection())).toEqual({
      canPreview: true,
      canApply: true,
      reason: "",
    });
  });

  it("refuses on Linux and names the alternative", () => {
    const capability = capabilityFor(detection({ platform: "linux" }));
    expect(capability.canApply).toBe(false);
    expect(capability.reason).toContain("macOS only");
    expect(capability.reason).toContain("SlimBrave Neo script");
  });

  it("refuses on Windows", () => {
    expect(capabilityFor(detection({ platform: "windows" })).canApply).toBe(false);
  });

  it("refuses when Brave was not found", () => {
    const capability = capabilityFor(
      detection({ found: false, channels: [] }),
    );
    expect(capability.canApply).toBe(false);
    expect(capability.reason).toContain("No Brave install was found");
  });

  it("refuses when detection reports channels but found is false", () => {
    expect(capabilityFor(detection({ found: false })).canApply).toBe(false);
  });

  it("blocks the first step when the platform is unsupported", () => {
    const state = reduce(initialState, {
      type: "detection.loaded",
      report: detection({ platform: "linux" }),
    });
    expect(canAdvance(state).ok).toBe(false);
    expect(canAdvance(state).reason).toContain("macOS only");
  });

  it("blocks apply on an unsupported platform even with a confirmed preview", () => {
    // The strongest form of the check: everything else is in order.
    const state = {
      ...run(
        initialState,
        { type: "detection.loaded", report: detection() },
        { type: "catalog.loaded", catalog: catalog() },
        { type: "channel.toggled", channelId: "beta" },
        { type: "step.next" },
        { type: "step.next" },
        { type: "preview.loaded", report: preview() },
        { type: "confirmation.set", confirmed: true },
      ),
      detection: detection({ platform: "windows" }),
    };
    expect(state.confirmed).toBe(true);
    expect(canApply(state).ok).toBe(false);
    expect(canApply(state).reason).toContain("macOS only");
  });
});

describe("the last step is a terminus", () => {
  it("cannot advance", () => {
    const state = { ...initialState, step: "done" as const };
    expect(canAdvance(state).ok).toBe(false);
  });

  it("cannot go back", () => {
    const state = { ...initialState, step: "done" as const };
    expect(reduce(state, { type: "step.back" }).step).toBe("done");
  });
});
