import {
  pollForToken as sdkPollForToken,
  startDeviceFlow as sdkStartDeviceFlow,
  isOpenHandsCloudHost as sdkIsOpenHandsCloudHost,
} from "@openhands/typescript-client/client/device-flow-client";
import type {
  DeviceAuthorizationResponse,
  DeviceTokenResponse,
  PollDeviceTokenOptions,
} from "@openhands/typescript-client/client/device-flow-client";
import { AGENT_CANVAS_CLIENT_HEADERS } from "./client-source";

export { DeviceFlowError } from "@openhands/typescript-client/client/device-flow-client";

/**
 * The vendored SDK only recognizes `all-hands.dev` / `openhands.dev`, so a
 * self-hosted MBILINYI Cloud host would be misclassified as a local agent
 * server. Treat the MBILINYI domains as Cloud hosts as well.
 */
export function isOpenHandsCloudHost(host: string): boolean {
  try {
    const trimmed = host.trim().toLowerCase();
    const withProtocol = /^https?:\/\//i.test(trimmed)
      ? trimmed
      : `https://${trimmed}`;
    const hostname = new URL(withProtocol).hostname;
    if (
      hostname.endsWith(".mbilinyi.dev") ||
      hostname === "mbilinyi.dev"
    ) {
      return true;
    }
  } catch {
    // Fall through to the SDK's own validation for malformed hosts.
  }

  return sdkIsOpenHandsCloudHost(host);
}

export function startDeviceFlow(
  host: string,
): Promise<DeviceAuthorizationResponse> {
  return sdkStartDeviceFlow(host, { headers: AGENT_CANVAS_CLIENT_HEADERS });
}

export function pollForToken(
  host: string,
  deviceCode: string,
  options: PollDeviceTokenOptions,
): Promise<DeviceTokenResponse> {
  return sdkPollForToken(host, deviceCode, {
    ...options,
    headers: AGENT_CANVAS_CLIENT_HEADERS,
  });
}

export type {
  DeviceAuthorizationResponse,
  DeviceTokenResponse,
  PollDeviceTokenOptions as PollOptions,
} from "@openhands/typescript-client/client/device-flow-client";
