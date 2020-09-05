use mockall::predicate::*;
use mockall::*;

pub struct AirtrafficController {
    airport_max_capacity: usize,
    airport_capacity: usize,
    plane_ids: Vec<u8>,
    weather_service: Box<dyn WeatherService>,
}

impl AirtrafficController {
    fn new(weather_service: Box<dyn WeatherService>, initial_planes: Vec<u8>) -> Self {
        Self {
            airport_capacity: initial_planes.len(),
            airport_max_capacity: 100,
            plane_ids: initial_planes,
            weather_service,
        }
    }

    fn allow_landing(&mut self, plane: &Plane) -> ControllerResponse {
        match self.check_weather() {
            (Weather::Stormy, _) => {
                return ControllerResponse::RejectLanding;
            }
            (_, _) => {}
        };

        if self.plane_ids.contains(&plane.id) {
            return ControllerResponse::RejectLanding;
        }

        if self.airport_capacity + 1 > self.airport_max_capacity {
            return ControllerResponse::Redirect;
        }

        self.add_plane(plane.id);
        ControllerResponse::AcceptLanding
    }

    fn allow_takeoff(&mut self, plane: &Plane) -> ControllerResponse {
        match self.check_weather() {
            (Weather::Stormy, _) => {
                return ControllerResponse::RejectTakeoff;
            }
            (_, _) => {}
        };

        match plane.state {
            PlaneState::Airborn => return ControllerResponse::RejectTakeoff,
            PlaneState::Landed => {}
        };

        self.remove_plane(&plane.id);

        ControllerResponse::AllowTakeoff
    }

    fn has_plane(&self, plane: &Plane) -> bool {
        self.plane_ids.iter().position(|&x| x == plane.id) != None
    }

    fn add_plane(&mut self, id: u8) {
        self.plane_ids.push(id);
        self.airport_capacity += 1
    }

    fn remove_plane(&mut self, id: &u8) {
        self.plane_ids
            .remove(self.plane_ids.iter().position(|x| x == id).unwrap());
        self.airport_capacity -= 1;
    }

    fn set_max_capacity(&mut self, max: usize) {
        self.airport_max_capacity = max;
    }

    fn check_weather(&self) -> (Weather, i8) {
        self.weather_service.get_weather()
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum ControllerResponse {
    AcceptLanding,
    RejectLanding,
    Redirect,
    AllowTakeoff,
    RejectTakeoff,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct Plane {
    id: u8,
    state: PlaneState,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum PlaneState {
    Landed,
    Airborn,
}

impl Plane {
    pub fn request_takeoff(&mut self, controller: &mut AirtrafficController) -> ControllerResponse {
        if let ControllerResponse::RejectTakeoff = controller.allow_takeoff(self) {
            return ControllerResponse::RejectTakeoff;
        };
        self.state = PlaneState::Airborn;
        ControllerResponse::AllowTakeoff
    }

    pub fn request_landing(&mut self, controller: &mut AirtrafficController) -> ControllerResponse {
        match controller.allow_landing(self) {
            ControllerResponse::RejectLanding => {
                return ControllerResponse::RejectLanding;
            }
            ControllerResponse::Redirect => {
                return ControllerResponse::Redirect;
            }
            _ => {}
        };

        self.state = PlaneState::Landed;
        ControllerResponse::AcceptLanding
    }
}

#[automock]
pub trait WeatherService {
    fn get_weather(&self) -> (Weather, i8);
}

#[derive(Clone)]
pub enum Weather {
    Clear,
    Cloudy,
    Sunny,
    Stormy,
    Raining,
    Snowing,
    Hailing,
}

#[cfg(test)]
mod test {
    use super::*;

    // As an air traffic controller
    // So I can get passengers to a destination
    // I want to instruct a plane to land at an airport
    #[test]
    fn plane_can_land() {
        let mut mock = Box::new(MockWeatherService::new());
        mock.expect_get_weather().return_const((Weather::Clear, 10));

        let mut controller = AirtrafficController::new(mock, vec![]);
        let mut plane = Plane {
            id: 1,
            state: PlaneState::Airborn,
        };

        assert_eq!(false, controller.has_plane(&plane));
        plane.request_landing(&mut controller);
        assert_eq!(true, controller.has_plane(&plane));
    }

    // As an air traffic controller
    // So I can make sure there are no collisions
    // I want to be sure a plane can't request to land if it already has
    #[test]
    fn plane_already_landed() {
        let mut mock = Box::new(MockWeatherService::new());
        mock.expect_get_weather().return_const((Weather::Clear, 10));

        let mut controller = AirtrafficController::new(mock, vec![1]);
        let mut plane = Plane {
            id: 1,
            state: PlaneState::Landed,
        };

        assert_eq!(true, controller.has_plane(&plane));
        plane.request_landing(&mut controller);
        assert_eq!(true, controller.has_plane(&plane));
    }

    // As an air traffic controller
    // So I can get passengers on the way to their destination
    // I want to instruct a plane to take off from an airport and confirm that it is no longer in the airport
    #[test]
    fn plane_can_take_off() {
        let mut mock = Box::new(MockWeatherService::new());
        mock.expect_get_weather().return_const((Weather::Clear, 10));

        let mut controller = AirtrafficController::new(mock, vec![1]);
        let mut plane = Plane {
            id: 1,
            state: PlaneState::Landed,
        };

        assert_eq!(PlaneState::Landed, plane.state);
        assert_eq!(true, controller.has_plane(&plane));

        plane.request_takeoff(&mut controller);

        assert_eq!(PlaneState::Airborn, plane.state);
        assert_eq!(false, controller.has_plane(&plane));
    }

    // As an air traffic controller
    // To ensure safety
    // I want to prevent landing when the airport is full
    #[test]
    fn plane_will_redirect_if_airport_is_full() {
        let mut mock = Box::new(MockWeatherService::new());
        mock.expect_get_weather().return_const((Weather::Clear, 10));

        let mut controller = AirtrafficController::new(mock, vec![]);
        let mut plane = Plane {
            id: 1,
            state: PlaneState::Airborn,
        };

        assert_eq!(false, controller.has_plane(&plane));

        controller.set_max_capacity(0);
        assert_eq!(
            ControllerResponse::Redirect,
            plane.request_landing(&mut controller)
        );
    }

    // As the system designer
    // So that the software can be used for many different airports
    // I would like a default airport capacity that can be overridden as appropriate
    #[test]
    fn airport_has_overridable_default_capacity() {
        let mock = Box::new(MockWeatherService::new());
        let mut controller = AirtrafficController::new(mock, vec![]);

        assert_eq!(100, controller.airport_max_capacity);
        controller.set_max_capacity(0);
        assert_eq!(0, controller.airport_max_capacity);
    }

    // As an air traffic controller
    // To ensure safety
    // I want to prevent takeoff when weather is stormy
    #[test]
    fn prevent_takeoff_during_storm() {
        let mut mock = Box::new(MockWeatherService::new());
        mock.expect_get_weather()
            .return_const((Weather::Stormy, -10));

        let mut controller = AirtrafficController::new(mock, vec![1]);
        let mut plane = Plane {
            id: 1,
            state: PlaneState::Landed,
        };

        assert_eq!(true, controller.has_plane(&plane));
        assert_eq!(PlaneState::Landed, plane.state);

        assert_eq!(
            ControllerResponse::RejectTakeoff,
            plane.request_takeoff(&mut controller)
        );

        assert_eq!(true, controller.has_plane(&plane));
        assert_eq!(PlaneState::Landed, plane.state);
    }

    // As an air traffic controller
    // To ensure safety
    // I want to prevent landing when weather is stormy
    #[test]
    fn prevent_landing_during_storm() {
        let mut mock = Box::new(MockWeatherService::new());
        mock.expect_get_weather()
            .return_const((Weather::Stormy, -10));

        let mut controller = AirtrafficController::new(mock, vec![]);
        let mut plane = Plane {
            id: 1,
            state: PlaneState::Airborn,
        };

        assert_eq!(false, controller.has_plane(&plane));
        assert_eq!(PlaneState::Airborn, plane.state);

        assert_eq!(
            ControllerResponse::RejectLanding,
            plane.request_landing(&mut controller)
        );

        assert_eq!(false, controller.has_plane(&plane));
        assert_eq!(PlaneState::Airborn, plane.state);
    }
}
