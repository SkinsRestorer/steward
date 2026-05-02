import type { BotConfig } from "@/bot-config";
import jarvisBotConfig from "./jarvis";
import stewardBotConfig from "./steward";

export const botConfigs: BotConfig[] = [stewardBotConfig, jarvisBotConfig];
