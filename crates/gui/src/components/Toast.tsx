import { useEffect, useState } from "react";
import { useApp } from "../lib/store";

/** 暗色提示丸(全局唯一,新消息重置计时) */
export default function Toast() {
  const { state } = useApp();
  const [show, setShow] = useState(false);
  const key = state.toast?.key;

  useEffect(() => {
    if (key === undefined) return;
    setShow(true);
    const t = setTimeout(() => setShow(false), 1600);
    return () => clearTimeout(t);
  }, [key]);

  return <div className={`toast${show ? " show" : ""}`}>{state.toast?.msg ?? ""}</div>;
}
