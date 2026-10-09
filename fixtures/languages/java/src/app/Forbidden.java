package app;
public class Forbidden {
    private infra.Store store;
    public String run() { return store.load(); }
}
