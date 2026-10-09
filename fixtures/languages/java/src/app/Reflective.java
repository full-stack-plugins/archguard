package app;
public class Reflective {
    public Object load(String name) throws Exception { return Class.forName(name).getDeclaredConstructor().newInstance(); }
    public Runnable defer() { return () -> System.out.println("dynamic"); }
}
