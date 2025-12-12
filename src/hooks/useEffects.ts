import { useState, useEffect } from "react";
import { EffectInfo } from "../types";
import { api } from "../services/api";
import { logger } from "../services/logger";

export function useEffects() {
  const [effects, setEffects] = useState<EffectInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    api.getEffects()
      .then((data) => {
        setEffects(data);
        setLoading(false);
      })
      .catch((err) => {
        logger.error("effects.fetch_failed", {}, err);
        setError(err);
        setLoading(false);
      });
  }, []);

  const applyEffect = async (port: string, effectId: string) => {
    try {
      await api.setEffect(port, effectId);
      return true;
    } catch (error) {
      logger.error("effects.apply_failed", { port, effectId }, error);
      return false;
    }
  };

  return { effects, loading, error, applyEffect };
}

