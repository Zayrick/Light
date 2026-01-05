import { useState, useEffect } from "react";
import { EffectInfo } from "../types";
import { api } from "../services/api";
import { logger } from "../services/logger";
import { sortEffects } from "../utils/effectsSort";

export function useEffects() {
  const [effects, setEffects] = useState<EffectInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    api.getEffects()
      .then((data) => {
        setEffects(sortEffects(data));
        setLoading(false);
      })
      .catch((err) => {
        logger.error("effects.fetch_failed", {}, err);
        setError(err);
        setLoading(false);
      });
  }, []);

  return { effects, loading, error };
}

