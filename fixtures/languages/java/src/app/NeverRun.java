package app;
public class NeverRun {
    static { if (true) throw new RuntimeException("candidate class was executed"); }
    public int value() { return 1; }
}
