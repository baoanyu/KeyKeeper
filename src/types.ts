export type PlanType = 'pay_as_you_go' | 'coding_plan' | 'subscription';
export type QuotaUnit = 'cny' | 'tokens' | 'seconds' | 'unknown';

export interface QuotaInfo {
  provider_name: string;
  plan_type: PlanType;
  quota_unit: QuotaUnit;
  total: number;
  remaining: number;
  is_success: boolean;
  error_msg?: string;
}

export const PROVIDERS = ['DeepSeek', 'ZhipuAI', 'Qoder', 'Volcano'] as const;
export type ProviderName = typeof PROVIDERS[number];
