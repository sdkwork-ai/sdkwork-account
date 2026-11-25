export interface AuthorizeWalletBillingRequest {
  organizationId?: string | null;
  ownerUserId: string;
  apiKeyId?: string | null;
  /** Estimated charge for the hypothetical request, in token-bank points. */
  estimatedPoints: string;
  walletOwnerType?: 'USER' | 'ORGANIZATION' | null;
  walletOwnerId?: string | null;
}
